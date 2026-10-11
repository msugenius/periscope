mod hotkey_runtime;
mod hotkeys;
mod overlay;
mod persistence;
mod settings;
mod updater;

use hotkeys::HotkeyController;
use overlay::OverlayController;
use persistence::{load_settings, persist_settings};
use serde::Serialize;
use settings::{AppSettings, CrosshairSettings, HotkeySettings, Preset, VisualSettings};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use updater::UpdaterState;

struct AppState {
    settings: Mutex<AppSettings>,
    preview: Mutex<VisualSettings>,
    settings_path: PathBuf,
    overlay: OverlayController,
    hotkeys: HotkeyController,
    quitting: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsView {
    #[serde(flatten)]
    crosshair: CrosshairSettings,
    active_preset: String,
    presets: Vec<Preset>,
    hotkeys: HotkeySettings,
    hotkey_errors: BTreeMap<String, String>,
    hide_when_ads: bool,
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> SettingsView {
    let persisted = state.settings.lock().expect("settings lock poisoned");
    SettingsView {
        crosshair: persisted.crosshair.clone(),
        active_preset: persisted.library.active_preset.clone(),
        presets: persisted.library.presets.clone(),
        hotkeys: state.hotkeys.settings(),
        hotkey_errors: state.hotkeys.errors(),
        hide_when_ads: persisted.hide_when_ads,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PresetState {
    active_preset: String,
    presets: Vec<Preset>,
    #[serde(flatten)]
    crosshair: CrosshairSettings,
}

fn mutate_library(
    state: &AppState,
    change: impl FnOnce(&mut crosshair_core::PresetLibrary) -> Result<(), String>,
) -> Result<PresetState, String> {
    let mut persisted = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    let mut next = persisted.clone();
    change(&mut next.library)?;
    next.crosshair = next
        .library
        .active()
        .settings
        .crosshair(next.crosshair.enabled);
    persist_settings(&state.settings_path, &next)?;
    *persisted = next.clone();
    let mut preview = state.preview.lock().map_err(|_| "preview lock poisoned")?;
    *preview = next.crosshair.visual.clone();
    state.overlay.update(next.crosshair.clone());
    Ok(PresetState {
        active_preset: next.library.active_preset,
        presets: next.library.presets,
        crosshair: next.crosshair,
    })
}

#[tauri::command]
fn preview_settings(
    settings: VisualSettings,
    state: State<'_, AppState>,
) -> Result<VisualSettings, String> {
    let settings = settings.validated();
    let persisted = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    let mut preview = state.preview.lock().map_err(|_| "preview lock poisoned")?;
    *preview = settings.clone();
    state
        .overlay
        .update(settings.crosshair(persisted.crosshair.enabled));
    Ok(settings)
}

#[tauri::command]
fn cancel_preview(state: State<'_, AppState>) -> Result<(), String> {
    let persisted = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    let mut preview = state.preview.lock().map_err(|_| "preview lock poisoned")?;
    *preview = persisted.crosshair.visual.clone();
    state.overlay.update(persisted.crosshair.clone());
    Ok(())
}

#[tauri::command]
fn save_preset_settings(
    preset: String,
    settings: VisualSettings,
    state: State<'_, AppState>,
) -> Result<PresetState, String> {
    mutate_library(state.inner(), |library| {
        if library.active_preset != preset {
            return Err("Active preset changed; reload before saving.".into());
        }
        library.save_active(settings);
        Ok(())
    })
}

#[tauri::command]
fn select_preset(preset: String, state: State<'_, AppState>) -> Result<PresetState, String> {
    mutate_library(state.inner(), |library| {
        library.select(&preset)?;
        Ok(())
    })
}

#[tauri::command]
fn create_preset(name: Option<String>, state: State<'_, AppState>) -> Result<PresetState, String> {
    mutate_library(state.inner(), |library| {
        let id = library.create(name.as_deref())?;
        library.select(&id)?;
        Ok(())
    })
}

#[tauri::command]
fn clone_preset(preset: String, state: State<'_, AppState>) -> Result<PresetState, String> {
    mutate_library(state.inner(), |library| {
        let id = library.clone_preset(&preset)?;
        library.select(&id)?;
        Ok(())
    })
}

#[tauri::command]
fn rename_preset(
    preset: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<PresetState, String> {
    mutate_library(state.inner(), |library| library.rename(&preset, &name))
}

#[tauri::command]
fn delete_preset(preset: String, state: State<'_, AppState>) -> Result<PresetState, String> {
    mutate_library(state.inner(), |library| library.delete(&preset))
}

#[tauri::command]
fn export_preset(preset: String, state: State<'_, AppState>) -> Result<String, String> {
    let persisted = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    persisted.library.export(&preset)
}

#[tauri::command]
fn import_preset(code: String, state: State<'_, AppState>) -> Result<PresetState, String> {
    mutate_library(state.inner(), |library| {
        let id = library.import(&code)?;
        library.select(&id)?;
        Ok(())
    })
}

#[tauri::command]
fn set_crosshair_enabled(enabled: bool, state: State<'_, AppState>) -> Result<bool, String> {
    let mut persisted = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    let mut next = persisted.clone();
    next.crosshair.enabled = enabled;
    persist_settings(&state.settings_path, &next)?;
    *persisted = next;
    let preview = state.preview.lock().map_err(|_| "preview lock poisoned")?;
    state.overlay.update(preview.crosshair(enabled));
    Ok(enabled)
}

fn apply_hotkeys(state: &AppState, hotkeys: HotkeySettings) -> Result<HotkeySettings, String> {
    let (accepted, rollback) = state.hotkeys.replace(hotkeys)?;
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    let previous = settings.hotkeys.clone();
    settings.hotkeys = accepted.clone();
    if let Err(error) = persist_settings(&state.settings_path, &settings) {
        settings.hotkeys = previous;
        drop(settings);
        let rollback_error = state.hotkeys.rollback(rollback).err();
        return Err(match rollback_error {
            Some(rollback_error) => format!(
                "Could not save hotkeys: {error}. The previous bindings could not be fully restored: {rollback_error}"
            ),
            None => format!("Could not save hotkeys: {error}"),
        });
    }
    Ok(accepted)
}

#[tauri::command]
fn update_hotkeys(
    hotkeys: HotkeySettings,
    state: State<'_, AppState>,
) -> Result<HotkeySettings, String> {
    apply_hotkeys(state.inner(), hotkeys)
}

#[tauri::command]
fn set_hotkey_recording(recording: bool, state: State<'_, AppState>) {
    state.hotkeys.set_recording(recording);
}

fn apply_hide_when_ads(app: &AppHandle, state: &AppState, enabled: bool) -> Result<bool, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    let previous = settings.hide_when_ads;
    if previous == enabled {
        return Ok(enabled);
    }
    state.overlay.set_hide_when_ads(enabled)?;
    settings.hide_when_ads = enabled;
    if let Err(error) = persist_settings(&state.settings_path, &settings) {
        settings.hide_when_ads = previous;
        let rollback_error = state.overlay.set_hide_when_ads(previous).err();
        return Err(match rollback_error {
            Some(rollback_error) => format!(
                "Could not save ADS setting: {error}. Could not restore mouse observation: {rollback_error}"
            ),
            None => format!("Could not save ADS setting: {error}"),
        });
    }
    let _ = app.emit("hide-when-ads-changed", enabled);
    Ok(enabled)
}

#[tauri::command]
fn set_hide_when_ads(
    app: AppHandle,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    apply_hide_when_ads(&app, state.inner(), enabled)
}

fn toggle_ads(app: &AppHandle) {
    let state = app.state::<AppState>();
    let enabled = match state.settings.lock() {
        Ok(settings) => !settings.hide_when_ads,
        Err(_) => return,
    };
    if let Err(error) = apply_hide_when_ads(app, state.inner(), enabled) {
        let _ = app.emit("hide-when-ads-error", error);
    }
}

#[tauri::command]
fn hide_settings(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.hotkeys.set_recording(false);
    cancel_preview(state)?;
    if let Some(window) = app.get_webview_window("main") {
        window.destroy().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn show_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("periScope")
        .inner_size(600.0, 770.0)
        .min_inner_size(600.0, 700.0)
        .center()
        .decorations(false)
        .transparent(true)
        .shadow(true)
        .build()?;
    Ok(())
}

fn quit_app(app: &AppHandle) {
    app.state::<AppState>()
        .quitting
        .store(true, Ordering::Release);
    app.exit(0);
}

fn toggle_crosshair(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(mut settings) = state.settings.lock() {
        settings.crosshair.enabled = !settings.crosshair.enabled;
        if let Ok(preview) = state.preview.lock() {
            state
                .overlay
                .update(preview.crosshair(settings.crosshair.enabled));
        }
        let _ = persist_settings(&state.settings_path, &settings);
        let _ = app.emit("crosshair-enabled-changed", settings.crosshair.enabled);
    }
}

fn tray_icon() -> tauri::image::Image<'static> {
    let size = 32_u32;
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let dx = x as i32 - 16;
            let dy = y as i32 - 16;
            let ring = (dx * dx + dy * dy >= 49 && dx * dx + dy * dy <= 81)
                || ((dx.abs() <= 1 && dy.abs() >= 10 && dy.abs() <= 14)
                    || (dy.abs() <= 1 && dx.abs() >= 10 && dx.abs() <= 14));
            if ring {
                let offset = ((y * size + x) * 4) as usize;
                rgba[offset] = 53;
                rgba[offset + 1] = 232;
                rgba[offset + 2] = 255;
                rgba[offset + 3] = 255;
            }
        }
    }
    tauri::image::Image::new_owned(rgba, size, size)
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open settings", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Toggle crosshair", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit periScope", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .icon(tray_icon())
        .tooltip("periScope - crosshair active")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_settings(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = show_settings(app);
            }
            "toggle" => {
                toggle_crosshair(app);
            }
            "quit" => {
                quit_app(app);
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.manage(UpdaterState::new(&app.package_info().version.to_string()));
            let settings_path = app
                .path()
                .app_config_dir()
                .map_err(|error| format!("could not resolve the settings directory: {error}"))?
                .join("settings.json");
            let mut settings = load_settings(&settings_path);
            let cleared_bindings = settings.hotkeys.clear_reserved_bindings();
            if !cleared_bindings.is_empty() {
                let _ = persist_settings(&settings_path, &settings);
            }
            let overlay = OverlayController::start(settings.crosshair.clone());
            if settings.hide_when_ads && overlay.set_hide_when_ads(true).is_err() {
                settings.hide_when_ads = false;
            }
            let hotkeys = HotkeyController::new(settings.hotkeys.clone());
            let preview = Mutex::new(settings.crosshair.visual.clone());
            app.manage(AppState {
                settings: Mutex::new(settings),
                preview,
                settings_path,
                overlay,
                hotkeys,
                quitting: AtomicBool::new(false),
            });
            if let Err(error) = app.state::<AppState>().hotkeys.start(app.handle()) {
                app.state::<AppState>()
                    .hotkeys
                    .set_error("configuration", error);
            }
            for (field, binding) in cleared_bindings {
                app.state::<AppState>().hotkeys.set_error(
                    field,
                    format!("Saved shortcut '{binding}' was cleared because Windows cannot expose it to apps. Choose another shortcut."),
                );
            }
            setup_tray(app.handle())
                .map_err(|error| format!("could not create the system tray icon: {error}"))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            preview_settings,
            cancel_preview,
            save_preset_settings,
            select_preset,
            create_preset,
            clone_preset,
            rename_preset,
            delete_preset,
            export_preset,
            import_preset,
            set_crosshair_enabled,
            update_hotkeys,
            set_hotkey_recording,
            set_hide_when_ads,
            hide_settings,
            updater::get_update_status,
            updater::start_update_check,
            updater::dismiss_update,
            updater::install_update
        ])
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.emit("settings-close-requested", ());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building periScope")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let state = app.state::<AppState>();
                if !state.quitting.load(Ordering::Acquire) {
                    api.prevent_exit();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::{SettingsView, load_settings, persist_settings};
    use crate::settings::AppSettings;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn test_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("periscope-{label}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn settings_view_exposes_presets_and_flat_crosshair_fields() {
        let settings = AppSettings::default();
        let view = SettingsView {
            crosshair: settings.crosshair,
            active_preset: settings.library.active_preset,
            presets: settings.library.presets,
            hotkeys: settings.hotkeys,
            hotkey_errors: Default::default(),
            hide_when_ads: settings.hide_when_ads,
        };
        let json = serde_json::to_value(view).unwrap();
        assert_eq!(json["color"], "#35E8FF");
        assert_eq!(json["activePreset"], "classic");
        assert_eq!(json["presets"].as_array().unwrap().len(), 3);
        assert!(json.get("visual").is_none());
    }

    #[test]
    fn missing_and_malformed_settings_load_defaults() {
        let directory = test_directory("load-defaults");
        let path = directory.join("settings.json");
        assert_eq!(load_settings(&path).hotkeys.toggle_crosshair, "F2");

        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, "{ definitely not json").unwrap();
        let recovered = load_settings(&path);
        assert_eq!(recovered.crosshair.visual.length, 10);
        assert_eq!(recovered.hotkeys.toggle_ads, "F5");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn atomic_persistence_replaces_existing_settings_and_round_trips() {
        let directory = test_directory("roundtrip");
        let path = directory.join("settings.json");
        let mut settings = AppSettings::default();
        persist_settings(&path, &settings).unwrap();

        settings.crosshair.visual.length = 37;
        settings
            .library
            .save_active(settings.crosshair.visual.clone());
        settings.hotkeys.toggle_crosshair = "Control+F2".into();
        settings.hotkeys.toggle_ads = "Control+F5".into();
        settings.hide_when_ads = true;
        persist_settings(&path, &settings).unwrap();
        let loaded = load_settings(&path);

        assert_eq!(loaded.crosshair.visual.length, 37);
        assert_eq!(loaded.hotkeys.toggle_crosshair, "Control+F2");
        assert_eq!(loaded.hotkeys.toggle_ads, "Control+F5");
        assert!(loaded.hide_when_ads);
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("\"length\": 37")
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn upgrades_the_previous_builtin_dot_to_a_true_dot() {
        let directory = test_directory("dot-upgrade");
        let path = directory.join("settings.json");
        fs::create_dir_all(&directory).unwrap();
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        let dot = &mut value["presets"][0]["settings"];
        dot.as_object_mut().unwrap().remove("dotOnly");
        dot["length"] = 1.into();
        dot["gap"] = 0.into();
        dot["dotSize"] = 7.into();
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();

        let loaded = load_settings(&path);
        let dot = &loaded.library.find("dot").unwrap().settings;
        assert!(dot.dot_only);
        assert_eq!(dot.dot_size, 7);
        assert_eq!(dot.length, 10);
        assert_eq!(dot.gap, 3);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn persistence_errors_do_not_replace_the_destination() {
        let directory = test_directory("persist-error");
        let destination = directory.join("settings.json");
        fs::create_dir_all(&destination).unwrap();

        let result = persist_settings(&destination, &AppSettings::default());

        assert!(result.is_err());
        assert!(destination.is_dir());
        fs::remove_dir_all(directory).unwrap();
    }
}
