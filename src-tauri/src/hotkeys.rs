use crate::hotkey_runtime::{KeyTransition, ShortcutDispatch};
use crate::settings::HotkeySettings;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Mutex,
};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleCrosshair,
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
    errors: BTreeMap<String, String>,
    dispatch: ShortcutDispatch,
}

pub struct HotkeyController {
    state: Mutex<RuntimeState>,
}

pub struct HotkeyRollback {
    snapshot: RuntimeSnapshot,
}

impl RuntimeSnapshot {
    fn capture(state: &RuntimeState) -> Self {
        Self {
            configured: state.configured.clone(),
            active: state.active.clone(),
            errors: state.errors.clone(),
        }
    }
}

impl HotkeyController {
    pub fn new(configured: HotkeySettings) -> Self {
        Self {
            state: Mutex::new(RuntimeState {
                configured,
                active: HashMap::new(),
                errors: BTreeMap::new(),
                dispatch: ShortcutDispatch::default(),
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
        let snapshot = RuntimeSnapshot::capture(&state);
        let was_recording = state.dispatch.is_recording();
        state.dispatch.set_recording(true);

        if let Err(error) =
            unregister_active(app, state.active.values().map(|(_, shortcut)| *shortcut))
        {
            state.dispatch.set_recording(was_recording);
            return Err(format!("Could not release the current hotkeys: {error}"));
        }
        state.active.clear();
        state.dispatch.clear();

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
                    state.dispatch.set_recording(was_recording);
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
        state.dispatch.set_recording(was_recording);
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
        state.dispatch.clear();
        if let Some(error) = restored.error {
            state.errors.insert("configuration".into(), error.clone());
            return Err(error);
        }
        Ok(())
    }

    pub fn handle_event(&self, shortcut: &Shortcut, event: ShortcutEvent) -> Option<HotkeyAction> {
        let mut state = self.state.lock().ok()?;
        let id = shortcut.id();
        let transition = match event.state {
            ShortcutState::Released => KeyTransition::Released,
            ShortcutState::Pressed => KeyTransition::Pressed,
        };
        state
            .dispatch
            .transition(id, transition)
            .then(|| state.active.get(&id).map(|(action, _)| *action))
            .flatten()
    }

    pub fn set_recording(&self, recording: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.dispatch.set_recording(recording);
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
    let mut shortcuts = Vec::new();
    if !settings.toggle_crosshair.is_empty() {
        shortcuts.push((
            "toggleCrosshair",
            HotkeyAction::ToggleCrosshair,
            parse_shortcut("Toggle crosshair", &settings.toggle_crosshair)?,
        ));
    }
    if !settings.close_app.is_empty() {
        shortcuts.push((
            "closeApp",
            HotkeyAction::CloseApp,
            parse_shortcut("Close app", &settings.close_app)?,
        ));
    }
    if !settings.show_settings.is_empty() {
        shortcuts.push((
            "showSettings",
            HotkeyAction::ShowSettings,
            parse_shortcut("Show settings", &settings.show_settings)?,
        ));
    }
    Ok(shortcuts)
}

fn parse_shortcut(label: &str, value: &str) -> Result<Shortcut, String> {
    value
        .parse::<Shortcut>()
        .map_err(|error| format!("{label} shortcut '{value}' is invalid: {error}"))
}

fn action_label(action: HotkeyAction) -> &'static str {
    match action {
        HotkeyAction::ToggleCrosshair => "Toggle crosshair",
        HotkeyAction::CloseApp => "Close app",
        HotkeyAction::ShowSettings => "Show settings",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey_runtime::{KeyTransition, ShortcutDispatch};

    #[test]
    fn parses_valid_shortcuts_and_reports_invalid_values() {
        let parsed = parse_pair(&HotkeySettings {
            close_app: "Control+F3".into(),
            show_settings: "F4".into(),
            ..HotkeySettings::default()
        })
        .unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].0, "toggleCrosshair");
        assert_eq!(parsed[0].1, HotkeyAction::ToggleCrosshair);
        assert_eq!(parsed[1].0, "closeApp");
        assert_eq!(parsed[1].1, HotkeyAction::CloseApp);
        assert!(parse_shortcut("Close app", "not-a-shortcut").is_err());
    }

    #[test]
    fn skips_unset_bindings() {
        let one_unset = parse_pair(&HotkeySettings {
            close_app: String::new(),
            ..HotkeySettings::default()
        })
        .unwrap();
        assert_eq!(one_unset.len(), 2);
        assert_eq!(one_unset[0].1, HotkeyAction::ToggleCrosshair);
        assert_eq!(one_unset[1].1, HotkeyAction::ShowSettings);
    }

    #[test]
    fn validation_rejects_duplicate_and_conflicting_shortcuts() {
        let conflict = HotkeySettings {
            toggle_crosshair: "Control+KeyQ".into(),
            close_app: "Control+KeyQ".into(),
            ..HotkeySettings::default()
        };
        assert!(conflict.validated().is_err());

        let duplicate_modifier = HotkeySettings {
            close_app: "Control+Control+KeyQ".into(),
            show_settings: "F4".into(),
            ..HotkeySettings::default()
        };
        assert!(duplicate_modifier.validated().is_err());
    }

    #[test]
    fn dispatch_suppresses_recording_and_deduplicates_press_release_cycles() {
        let mut dispatch = ShortcutDispatch::default();
        assert!(dispatch.transition(42, KeyTransition::Pressed));
        assert!(!dispatch.transition(42, KeyTransition::Pressed));
        assert!(!dispatch.transition(42, KeyTransition::Released));
        assert!(dispatch.transition(42, KeyTransition::Pressed));

        dispatch.set_recording(true);
        assert!(!dispatch.transition(7, KeyTransition::Pressed));
        dispatch.set_recording(false);
        assert!(dispatch.transition(7, KeyTransition::Pressed));
        dispatch.clear();
        assert!(dispatch.transition(42, KeyTransition::Pressed));
    }

    #[test]
    fn rollback_snapshot_is_isolated_from_later_runtime_mutation() {
        let close = parse_shortcut("Close app", "F3").unwrap();
        let state = RuntimeState {
            configured: HotkeySettings::default(),
            active: HashMap::from([(close.id(), (HotkeyAction::CloseApp, close))]),
            errors: BTreeMap::from([("configuration".into(), "original".into())]),
            dispatch: ShortcutDispatch::default(),
        };
        let snapshot = RuntimeSnapshot::capture(&state);

        let mut changed = state;
        changed.configured.close_app = "F5".into();
        changed.active.clear();
        changed.errors.clear();

        assert_eq!(snapshot.configured.close_app, "F3");
        assert_eq!(snapshot.active.len(), 1);
        assert_eq!(
            snapshot.errors.get("configuration").map(String::as_str),
            Some("original")
        );
    }
}
