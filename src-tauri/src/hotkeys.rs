use crate::hotkey_runtime::{KeyTransition, ShortcutDispatch};
use crate::settings::HotkeySettings;
use std::{
    collections::{BTreeMap, HashSet},
    ptr,
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread,
};
use tauri::{AppHandle, Emitter};
use windows_sys::Win32::{
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_HOME,
            VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT,
            VK_NUMLOCK, VK_NUMPAD0, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU,
            VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
        },
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, MSG,
            SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN,
            WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

const CONTROL: u8 = 1;
const ALT: u8 = 2;
const SHIFT: u8 = 4;
const SUPER: u8 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleCrosshair,
    ToggleAds,
}

#[derive(Debug, PartialEq, Eq)]
enum HookMessage {
    Action(HotkeyAction),
    Recorded(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Binding {
    action: HotkeyAction,
    key: u32,
    modifiers: u8,
}

#[derive(Clone)]
struct RuntimeSnapshot {
    configured: HotkeySettings,
    active: Vec<Binding>,
    errors: BTreeMap<String, String>,
}

struct RuntimeState {
    configured: HotkeySettings,
    active: Vec<Binding>,
    errors: BTreeMap<String, String>,
    dispatch: ShortcutDispatch,
    held_modifiers: HashSet<(u32, u32)>,
    hook_running: bool,
    recording_key: Option<u32>,
    stop_after_release: bool,
}

impl RuntimeState {
    fn modifiers(&self) -> u8 {
        self.held_modifiers
            .iter()
            .fold(0, |mask, (key, _)| mask | modifier_for_key(*key))
    }

    fn set_recording(&mut self, recording: bool) {
        if recording {
            self.stop_after_release = false;
            self.dispatch.set_recording(true);
        } else if self.recording_key.is_some() {
            self.stop_after_release = true;
        } else {
            self.dispatch.set_recording(false);
        }
    }

    #[cfg(test)]
    fn key_event(
        &mut self,
        key: u32,
        scan_code: u32,
        transition: KeyTransition,
    ) -> Option<HookMessage> {
        self.key_event_with_alt(key, scan_code, transition, false)
    }

    fn key_event_with_alt(
        &mut self,
        key: u32,
        scan_code: u32,
        transition: KeyTransition,
        alt_context: bool,
    ) -> Option<HookMessage> {
        if modifier_for_key(key) != 0 {
            match transition {
                KeyTransition::Pressed => {
                    self.held_modifiers.insert((key, scan_code));
                }
                KeyTransition::Released => {
                    self.held_modifiers.remove(&(key, scan_code));
                }
            }
            return None;
        }
        if matches!(transition, KeyTransition::Released) {
            self.dispatch.transition(key, transition);
            if self.recording_key == Some(key) {
                self.recording_key = None;
                if self.stop_after_release {
                    self.stop_after_release = false;
                    self.dispatch.set_recording(false);
                }
            }
            return None;
        }
        let modifiers = self.modifiers() | if alt_context { ALT } else { 0 };
        if self.dispatch.is_recording() {
            if self.recording_key.is_some() {
                return None;
            }
            let key_name = key_name(key)?;
            self.recording_key = Some(key);
            return Some(HookMessage::Recorded(format_shortcut(modifiers, &key_name)));
        }
        let action = self
            .active
            .iter()
            .find(|binding| binding.key == key && binding.modifiers == modifiers)
            .map(|binding| binding.action)?;
        self.dispatch
            .transition(key, transition)
            .then_some(HookMessage::Action(action))
    }
}

struct HookContext {
    state: Arc<Mutex<RuntimeState>>,
    actions: mpsc::Sender<HookMessage>,
}

static HOOK_CONTEXT: OnceLock<HookContext> = OnceLock::new();

pub struct HotkeyController {
    state: Arc<Mutex<RuntimeState>>,
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
            state: Arc::new(Mutex::new(RuntimeState {
                configured,
                active: Vec::new(),
                errors: BTreeMap::new(),
                dispatch: ShortcutDispatch::default(),
                held_modifiers: HashSet::new(),
                hook_running: false,
                recording_key: None,
                stop_after_release: false,
            })),
        }
    }

    pub fn start(&self, app: &AppHandle) -> Result<(), String> {
        let configured = self.settings();
        match configured.validated().and_then(|validated| {
            let bindings = parse_bindings(&validated)?;
            Ok((validated, bindings))
        }) {
            Ok((validated, bindings)) => {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| "Hotkey state is unavailable.")?;
                state.configured = validated;
                state.active = bindings;
            }
            Err(error) => self.set_error(
                "configuration",
                format!("Saved hotkeys are invalid. {error}"),
            ),
        }

        let (actions, receiver) = mpsc::channel();
        HOOK_CONTEXT
            .set(HookContext {
                state: Arc::clone(&self.state),
                actions,
            })
            .map_err(|_| "Keyboard observer is already running.".to_string())?;

        let handle = app.clone();
        thread::Builder::new()
            .name("periscope-hotkey-actions".into())
            .spawn(move || {
                for action in receiver {
                    match action {
                        HookMessage::Action(HotkeyAction::ToggleCrosshair) => {
                            crate::toggle_crosshair(&handle)
                        }
                        HookMessage::Action(HotkeyAction::ToggleAds) => crate::toggle_ads(&handle),
                        HookMessage::Recorded(shortcut) => {
                            let _ = handle.emit("hotkey-recorded", shortcut);
                        }
                    }
                }
            })
            .map_err(|error| format!("Could not start hotkey actions: {error}"))?;

        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("periscope-keyboard-observer".into())
            .spawn(move || run_keyboard_observer(ready_sender))
            .map_err(|error| format!("Could not start keyboard observer: {error}"))?;
        ready_receiver
            .recv()
            .map_err(|_| "Keyboard observer stopped during startup.".to_string())??;
        self.state
            .lock()
            .map_err(|_| "Hotkey state is unavailable.".to_string())?
            .hook_running = true;
        Ok(())
    }

    pub fn replace(
        &self,
        proposed: HotkeySettings,
    ) -> Result<(HotkeySettings, HotkeyRollback), String> {
        let proposed = proposed.validated()?;
        let bindings = parse_bindings(&proposed)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Hotkey state is unavailable.")?;
        if !state.hook_running {
            return Err("Keyboard observer is unavailable.".into());
        }
        let snapshot = RuntimeSnapshot::capture(&state);
        state.active = bindings;
        state.configured = proposed.clone();
        state.errors.clear();
        state.dispatch.clear();
        Ok((proposed, HotkeyRollback { snapshot }))
    }

    pub fn rollback(&self, rollback: HotkeyRollback) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Hotkey state is unavailable.")?;
        state.configured = rollback.snapshot.configured;
        state.active = rollback.snapshot.active;
        state.errors = rollback.snapshot.errors;
        state.dispatch.clear();
        Ok(())
    }

    pub fn set_recording(&self, recording: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.set_recording(recording);
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

    pub(crate) fn set_error(&self, field: &str, error: String) {
        if let Ok(mut state) = self.state.lock() {
            state.errors.insert(field.into(), error);
        }
    }
}

fn modifier_for_key(key: u32) -> u8 {
    match key as u16 {
        VK_CONTROL | VK_LCONTROL | VK_RCONTROL => CONTROL,
        VK_MENU | VK_LMENU | VK_RMENU => ALT,
        VK_SHIFT | VK_LSHIFT | VK_RSHIFT => SHIFT,
        VK_LWIN | VK_RWIN => SUPER,
        _ => 0,
    }
}

fn parse_bindings(settings: &HotkeySettings) -> Result<Vec<Binding>, String> {
    let mut bindings = Vec::new();
    for (value, action) in [
        (&settings.toggle_crosshair, HotkeyAction::ToggleCrosshair),
        (&settings.toggle_ads, HotkeyAction::ToggleAds),
    ] {
        if value.is_empty() {
            continue;
        }
        let (modifiers, key) = value.rsplit_once('+').unwrap_or(("", value));
        let mut mask = 0;
        for modifier in modifiers.split('+').filter(|part| !part.is_empty()) {
            mask |= match modifier {
                "Control" => CONTROL,
                "Alt" => ALT,
                "Shift" => SHIFT,
                "Super" => SUPER,
                _ => return Err(format!("Unsupported modifier in {value}.")),
            };
        }
        bindings.push(Binding {
            action,
            key: virtual_key(key).ok_or_else(|| format!("Unsupported key in {value}."))?,
            modifiers: mask,
        });
    }
    Ok(bindings)
}

fn virtual_key(key: &str) -> Option<u32> {
    if let Some(letter) = key.strip_prefix("Key")
        && letter.len() == 1
        && letter.as_bytes()[0].is_ascii_uppercase()
    {
        return Some(letter.as_bytes()[0] as u32);
    }
    if let Some(digit) = key.strip_prefix("Digit")
        && digit.len() == 1
        && digit.as_bytes()[0].is_ascii_digit()
    {
        return Some(digit.as_bytes()[0] as u32);
    }
    if let Some(digit) = key.strip_prefix("Numpad")
        && digit.len() == 1
        && digit.as_bytes()[0].is_ascii_digit()
    {
        return Some(u32::from(VK_NUMPAD0) + u32::from(digit.as_bytes()[0] - b'0'));
    }
    if let Some(number) = key
        .strip_prefix('F')
        .and_then(|part| part.parse::<u32>().ok())
        && (1..=24).contains(&number)
    {
        return Some(u32::from(VK_F1) + number - 1);
    }
    Some(u32::from(match key {
        "ArrowUp" => VK_UP,
        "ArrowDown" => VK_DOWN,
        "ArrowLeft" => VK_LEFT,
        "ArrowRight" => VK_RIGHT,
        "Backspace" => VK_BACK,
        "Delete" => VK_DELETE,
        "End" => VK_END,
        "Enter" => VK_RETURN,
        "Home" => VK_HOME,
        "Insert" => VK_INSERT,
        "PageDown" => VK_NEXT,
        "PageUp" => VK_PRIOR,
        "Space" => VK_SPACE,
        "Tab" => VK_TAB,
        "CapsLock" => VK_CAPITAL,
        "NumLock" => VK_NUMLOCK,
        "ScrollLock" => VK_SCROLL,
        _ => return None,
    }))
}

fn key_name(key: u32) -> Option<String> {
    if (b'A' as u32..=b'Z' as u32).contains(&key) {
        return Some(format!("Key{}", char::from_u32(key)?));
    }
    if (b'0' as u32..=b'9' as u32).contains(&key) {
        return Some(format!("Digit{}", char::from_u32(key)?));
    }
    if (u32::from(VK_NUMPAD0)..=u32::from(VK_NUMPAD0) + 9).contains(&key) {
        return Some(format!("Numpad{}", key - u32::from(VK_NUMPAD0)));
    }
    if (u32::from(VK_F1)..=u32::from(VK_F1) + 23).contains(&key) {
        return Some(format!("F{}", key - u32::from(VK_F1) + 1));
    }
    Some(
        match key as u16 {
            VK_UP => "ArrowUp",
            VK_DOWN => "ArrowDown",
            VK_LEFT => "ArrowLeft",
            VK_RIGHT => "ArrowRight",
            VK_BACK => "Backspace",
            VK_DELETE => "Delete",
            VK_END => "End",
            VK_RETURN => "Enter",
            VK_HOME => "Home",
            VK_INSERT => "Insert",
            VK_NEXT => "PageDown",
            VK_PRIOR => "PageUp",
            VK_SPACE => "Space",
            VK_TAB => "Tab",
            VK_CAPITAL => "CapsLock",
            VK_NUMLOCK => "NumLock",
            VK_SCROLL => "ScrollLock",
            VK_ESCAPE => "Escape",
            _ => return None,
        }
        .to_string(),
    )
}

fn format_shortcut(modifiers: u8, key: &str) -> String {
    let mut parts = Vec::new();
    for (flag, name) in [
        (CONTROL, "Control"),
        (ALT, "Alt"),
        (SHIFT, "Shift"),
        (SUPER, "Super"),
    ] {
        if modifiers & flag != 0 {
            parts.push(name);
        }
    }
    parts.push(key);
    parts.join("+")
}

fn run_keyboard_observer(ready: mpsc::SyncSender<Result<(), String>>) {
    let hook = unsafe {
        SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_proc),
            GetModuleHandleW(ptr::null()),
            0,
        )
    };
    if hook.is_null() {
        let _ = ready.send(Err(format!(
            "Could not observe the keyboard: {}",
            std::io::Error::last_os_error()
        )));
        return;
    }
    let _ = ready.send(Ok(()));
    let mut message: MSG = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    unsafe { UnhookWindowsHookEx(hook) };
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    if code >= 0 {
        let transition = match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(KeyTransition::Pressed),
            WM_KEYUP | WM_SYSKEYUP => Some(KeyTransition::Released),
            _ => None,
        };
        if let Some(transition) = transition
            && let Some(context) = HOOK_CONTEXT.get()
        {
            let event = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
            if let Ok(mut state) = context.state.lock()
                && let Some(action) = state.key_event_with_alt(
                    event.vkCode,
                    event.scanCode,
                    transition,
                    event.flags & LLKHF_ALTDOWN != 0,
                )
            {
                let _ = context.actions.send(action);
            }
        }
    }
    unsafe { CallNextHookEx(ptr::null_mut(), code, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(settings: HotkeySettings) -> RuntimeState {
        let validated = settings.validated().unwrap();
        RuntimeState {
            active: parse_bindings(&validated).unwrap(),
            configured: validated,
            errors: BTreeMap::new(),
            dispatch: ShortcutDispatch::default(),
            held_modifiers: HashSet::new(),
            hook_running: true,
            recording_key: None,
            stop_after_release: false,
        }
    }

    #[test]
    fn parses_passive_bindings_and_skips_unset_actions() {
        let bindings = parse_bindings(&HotkeySettings {
            toggle_crosshair: "KeyS".into(),
            toggle_ads: "Alt+Tab".into(),
        })
        .unwrap();
        assert_eq!(bindings[0].key, u32::from(b'S'));
        assert_eq!(bindings[0].modifiers, 0);
        assert_eq!(bindings[1].key, u32::from(VK_TAB));
        assert_eq!(bindings[1].modifiers, ALT);
        assert!(
            parse_bindings(&HotkeySettings {
                toggle_crosshair: String::new(),
                toggle_ads: String::new(),
            })
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn key_events_match_s_and_alt_tab_once_per_press() {
        let mut state = runtime(HotkeySettings {
            toggle_crosshair: "KeyS".into(),
            toggle_ads: "Alt+Tab".into(),
        });
        assert_eq!(
            state.key_event(u32::from(b'S'), 31, KeyTransition::Pressed),
            Some(HookMessage::Action(HotkeyAction::ToggleCrosshair))
        );
        assert_eq!(
            state.key_event(u32::from(b'S'), 31, KeyTransition::Pressed),
            None
        );
        assert_eq!(
            state.key_event(u32::from(b'S'), 31, KeyTransition::Released),
            None
        );
        assert_eq!(
            state.key_event(u32::from(VK_MENU), 56, KeyTransition::Pressed),
            None
        );
        assert_eq!(
            state.key_event(u32::from(VK_TAB), 15, KeyTransition::Pressed),
            Some(HookMessage::Action(HotkeyAction::ToggleAds))
        );
        assert_eq!(
            state.key_event(u32::from(VK_TAB), 15, KeyTransition::Released),
            None
        );
        assert_eq!(
            state.key_event(u32::from(VK_MENU), 56, KeyTransition::Released),
            None
        );
        assert_eq!(
            state.key_event(u32::from(b'S'), 31, KeyTransition::Pressed),
            Some(HookMessage::Action(HotkeyAction::ToggleCrosshair))
        );
    }

    #[test]
    fn lock_keys_and_windows_combinations_are_supported() {
        let mut state = runtime(HotkeySettings {
            toggle_crosshair: "CapsLock".into(),
            toggle_ads: "Super+KeyR".into(),
        });
        assert_eq!(
            state.key_event(u32::from(VK_CAPITAL), 58, KeyTransition::Pressed),
            Some(HookMessage::Action(HotkeyAction::ToggleCrosshair))
        );
        assert_eq!(
            state.key_event(u32::from(VK_CAPITAL), 58, KeyTransition::Released),
            None
        );
        assert_eq!(
            state.key_event(u32::from(VK_LWIN), 91, KeyTransition::Pressed),
            None
        );
        assert_eq!(
            state.key_event(u32::from(b'R'), 19, KeyTransition::Pressed),
            Some(HookMessage::Action(HotkeyAction::ToggleAds))
        );
    }

    #[test]
    fn recording_suspends_actions_without_losing_modifier_state() {
        let mut state = runtime(HotkeySettings {
            toggle_crosshair: "Control+KeyS".into(),
            toggle_ads: "F5".into(),
        });
        state.set_recording(true);
        assert_eq!(
            state.key_event(u32::from(VK_CONTROL), 29, KeyTransition::Pressed),
            None
        );
        assert_eq!(
            state.key_event(u32::from(b'S'), 31, KeyTransition::Pressed),
            Some(HookMessage::Recorded("Control+KeyS".into()))
        );
        assert_eq!(
            state.key_event(u32::from(b'S'), 31, KeyTransition::Pressed),
            None
        );
        state.set_recording(false);
        assert!(state.dispatch.is_recording());
        assert_eq!(
            state.key_event(u32::from(b'S'), 31, KeyTransition::Released),
            None
        );
        assert!(!state.dispatch.is_recording());
        assert_eq!(
            state.key_event(u32::from(b'S'), 31, KeyTransition::Pressed),
            Some(HookMessage::Action(HotkeyAction::ToggleCrosshair))
        );
    }

    #[test]
    fn native_recording_sees_alt_tab_and_allows_another_attempt_after_release() {
        let mut state = runtime(HotkeySettings::default());
        state.set_recording(true);
        assert_eq!(
            state.key_event(u32::from(VK_MENU), 56, KeyTransition::Pressed),
            None
        );
        assert_eq!(
            state.key_event(u32::from(VK_TAB), 15, KeyTransition::Pressed),
            Some(HookMessage::Recorded("Alt+Tab".into()))
        );
        assert_eq!(
            state.key_event(u32::from(VK_TAB), 15, KeyTransition::Pressed),
            None
        );
        assert_eq!(
            state.key_event(u32::from(VK_TAB), 15, KeyTransition::Released),
            None
        );
        assert!(state.dispatch.is_recording());
        assert_eq!(
            state.key_event(u32::from(VK_MENU), 56, KeyTransition::Released),
            None
        );
        assert_eq!(
            state.key_event(u32::from(VK_CAPITAL), 58, KeyTransition::Pressed),
            Some(HookMessage::Recorded("CapsLock".into()))
        );
    }

    #[test]
    fn alt_context_recovers_system_combo_when_alt_press_was_not_seen() {
        let mut state = runtime(HotkeySettings {
            toggle_crosshair: "Alt+Tab".into(),
            toggle_ads: "F5".into(),
        });
        assert_eq!(
            state.key_event_with_alt(u32::from(VK_TAB), 15, KeyTransition::Pressed, true),
            Some(HookMessage::Action(HotkeyAction::ToggleCrosshair))
        );
        assert_eq!(
            state.key_event(u32::from(VK_TAB), 15, KeyTransition::Released),
            None
        );
        state.set_recording(true);
        assert_eq!(
            state.key_event_with_alt(u32::from(VK_TAB), 15, KeyTransition::Pressed, true),
            Some(HookMessage::Recorded("Alt+Tab".into()))
        );
    }

    #[test]
    fn rollback_snapshot_restores_previous_bindings() {
        let mut state = runtime(HotkeySettings::default());
        state
            .errors
            .insert("configuration".into(), "original".into());
        let snapshot = RuntimeSnapshot::capture(&state);
        state.configured.toggle_ads = "F6".into();
        state.active.clear();
        state.errors.clear();
        assert_eq!(snapshot.configured.toggle_ads, "F5");
        assert_eq!(snapshot.active.len(), 2);
        assert_eq!(
            snapshot.errors.get("configuration").map(String::as_str),
            Some("original")
        );
    }
}
