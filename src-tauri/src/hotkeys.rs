use crate::settings::HotkeySettings;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Mutex,
};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    CloseApp,
    ShowSettings,
}

#[derive(Clone)]
struct RuntimeSnapshot {
    configured: HotkeySettings,
    active: HashMap<u32, (HotkeyAction, Shortcut)>,
    errors: BTreeMap<String, String>,
}

struct RuntimeState {
    configured: HotkeySettings,
    active: HashMap<u32, (HotkeyAction, Shortcut)>,
    pressed: HashSet<u32>,
    errors: BTreeMap<String, String>,
    recording: bool,
}

pub struct HotkeyController {
    state: Mutex<RuntimeState>,
}

pub struct HotkeyRollback {
    snapshot: RuntimeSnapshot,
}

impl HotkeyController {
    pub fn new(configured: HotkeySettings) -> Self {
        Self {
            state: Mutex::new(RuntimeState {
                configured,
                active: HashMap::new(),
                pressed: HashSet::new(),
                errors: BTreeMap::new(),
                recording: false,
            }),
        }
    }

    pub fn register_startup(&self, app: &AppHandle) {
        let configured = self.settings();
        let validated = match configured.validated() {
            Ok(settings) => settings,
            Err(error) => {
                self.set_error(
                    "configuration",
                    format!("Saved hotkeys are invalid. {error}"),
                );
                return;
            }
        };

        let shortcuts = match parse_pair(&validated) {
            Ok(shortcuts) => shortcuts,
            Err(error) => {
                self.set_error("configuration", error);
                return;
            }
        };

        let mut active = HashMap::new();
        let mut errors = BTreeMap::new();
        for (field, action, shortcut) in shortcuts {
            match app.global_shortcut().register(shortcut) {
                Ok(()) => {
                    active.insert(shortcut.id(), (action, shortcut));
                }
                Err(error) => {
                    errors.insert(
                        field.into(),
                        format!("{} is unavailable: {error}", action_label(action)),
                    );
                }
            }
        }

        let mut state = self.state.lock().expect("hotkey state lock poisoned");
        state.configured = validated;
        state.active = active;
        state.errors = errors;
    }

    pub fn replace(
        &self,
        app: &AppHandle,
        proposed: HotkeySettings,
    ) -> Result<(HotkeySettings, HotkeyRollback), String> {
        let proposed = proposed.validated()?;
        let proposed_shortcuts = parse_pair(&proposed)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Hotkey state is unavailable.".to_string())?;
        let snapshot = RuntimeSnapshot {
            configured: state.configured.clone(),
            active: state.active.clone(),
            errors: state.errors.clone(),
        };
        let was_recording = state.recording;
        state.recording = true;

        if let Err(error) =
            unregister_active(app, state.active.values().map(|(_, shortcut)| *shortcut))
        {
            state.recording = was_recording;
            return Err(format!("Could not release the current hotkeys: {error}"));
        }
        state.active.clear();
        state.pressed.clear();

        let mut newly_registered = Vec::new();
        for (field, action, shortcut) in proposed_shortcuts {
            match app.global_shortcut().register(shortcut) {
                Ok(()) => newly_registered.push((field, action, shortcut)),
                Err(error) => {
                    let _ = unregister_active(
                        app,
                        newly_registered.iter().map(|(_, _, shortcut)| *shortcut),
                    );
                    let rollback = restore_snapshot(app, &snapshot);
                    state.configured = snapshot.configured;
                    state.active = rollback.active;
                    state.errors = snapshot.errors;
                    state.recording = was_recording;
                    if let Some(rollback_error) = rollback.error {
                        state
                            .errors
                            .insert("configuration".into(), rollback_error.clone());
                        return Err(format!(
                            "{} is unavailable: {error}. The previous hotkeys could not be fully restored: {rollback_error}",
                            action_label(action)
                        ));
                    }
                    return Err(format!("{} is unavailable: {error}", action_label(action)));
                }
            }
        }

        state.active = newly_registered
            .into_iter()
            .map(|(_, action, shortcut)| (shortcut.id(), (action, shortcut)))
            .collect();
        state.configured = proposed.clone();
        state.errors.clear();
        state.recording = was_recording;
        Ok((proposed, HotkeyRollback { snapshot }))
    }

    pub fn rollback(&self, app: &AppHandle, rollback: HotkeyRollback) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Hotkey state is unavailable.".to_string())?;
        let _ = unregister_active(app, state.active.values().map(|(_, shortcut)| *shortcut));
        let restored = restore_snapshot(app, &rollback.snapshot);
        state.configured = rollback.snapshot.configured;
        state.active = restored.active;
        state.errors = rollback.snapshot.errors;
        state.pressed.clear();
        if let Some(error) = restored.error {
            state.errors.insert("configuration".into(), error.clone());
            return Err(error);
        }
        Ok(())
    }

    pub fn handle_event(&self, shortcut: &Shortcut, event: ShortcutEvent) -> Option<HotkeyAction> {
        let mut state = self.state.lock().ok()?;
        let id = shortcut.id();
        match event.state {
            ShortcutState::Released => {
                state.pressed.remove(&id);
                None
            }
            ShortcutState::Pressed => {
                if !state.pressed.insert(id) || state.recording {
                    return None;
                }
                state.active.get(&id).map(|(action, _)| *action)
            }
        }
    }

    pub fn set_recording(&self, recording: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.recording = recording;
        }
    }

    pub fn settings(&self) -> HotkeySettings {
        self.state
            .lock()
            .map(|state| state.configured.clone())
            .unwrap_or_default()
    }

    pub fn errors(&self) -> BTreeMap<String, String> {
        self.state
            .lock()
            .map(|state| state.errors.clone())
            .unwrap_or_else(|_| {
                BTreeMap::from([(
                    "configuration".into(),
                    "Hotkey status is unavailable.".into(),
                )])
            })
    }

    fn set_error(&self, field: &str, error: String) {
        if let Ok(mut state) = self.state.lock() {
            state.errors.insert(field.into(), error);
        }
    }
}

struct RollbackResult {
    active: HashMap<u32, (HotkeyAction, Shortcut)>,
    error: Option<String>,
}

fn restore_snapshot(app: &AppHandle, snapshot: &RuntimeSnapshot) -> RollbackResult {
    let mut active = HashMap::new();
    let mut errors = Vec::new();
    for (action, shortcut) in snapshot.active.values() {
        match app.global_shortcut().register(*shortcut) {
            Ok(()) => {
                active.insert(shortcut.id(), (*action, *shortcut));
            }
            Err(error) => errors.push(format!("{}: {error}", action_label(*action))),
        }
    }
    RollbackResult {
        active,
        error: (!errors.is_empty()).then(|| errors.join("; ")),
    }
}

fn unregister_active(
    app: &AppHandle,
    shortcuts: impl IntoIterator<Item = Shortcut>,
) -> Result<(), String> {
    for shortcut in shortcuts {
        app.global_shortcut()
            .unregister(shortcut)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn parse_pair(
    settings: &HotkeySettings,
) -> Result<Vec<(&'static str, HotkeyAction, Shortcut)>, String> {
    Ok(vec![
        (
            "closeApp",
            HotkeyAction::CloseApp,
            parse_shortcut("Close app", &settings.close_app)?,
        ),
        (
            "showSettings",
            HotkeyAction::ShowSettings,
            parse_shortcut("Show settings", &settings.show_settings)?,
        ),
    ])
}

fn parse_shortcut(label: &str, value: &str) -> Result<Shortcut, String> {
    value
        .parse::<Shortcut>()
        .map_err(|error| format!("{label} shortcut '{value}' is invalid: {error}"))
}

fn action_label(action: HotkeyAction) -> &'static str {
    match action {
        HotkeyAction::CloseApp => "Close app",
        HotkeyAction::ShowSettings => "Show settings",
    }
}
