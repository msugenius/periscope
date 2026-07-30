mod hotkeys;
mod overlay;
mod settings;

use hotkeys::{HotkeyAction, HotkeyController};
use overlay::OverlayController;
use serde::Serialize;
use settings::{AppSettings, CrosshairSettings, HotkeySettings};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{
    AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

struct AppState {
    settings: Mutex<AppSettings>,
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
    hotkeys: HotkeySettings,
    hotkey_errors: BTreeMap<String, String>,
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> SettingsView {
    let crosshair = state
        .settings
        .lock()
        .expect("settings lock poisoned")
        .crosshair
        .clone();
    SettingsView {
        crosshair,
        hotkeys: state.hotkeys.settings(),
        hotkey_errors: state.hotkeys.errors(),
    }
}

#[tauri::command]
fn update_settings(
    settings: CrosshairSettings,
    state: State<'_, AppState>,
) -> Result<CrosshairSettings, String> {
    let settings = settings.validated();
    state.overlay.update(settings.clone());
    let mut persisted = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    persisted.crosshair = settings.clone();
    persist_settings(&state.settings_path, &persisted)?;
    Ok(settings)
}

#[tauri::command]
fn reset_settings(state: State<'_, AppState>) -> Result<CrosshairSettings, String> {
    update_settings(CrosshairSettings::default(), state)
}

fn apply_hotkeys(
    app: &AppHandle,
    state: &AppState,
    hotkeys: HotkeySettings,
) -> Result<HotkeySettings, String> {
    let (accepted, rollback) = state.hotkeys.replace(app, hotkeys)?;
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    let previous = settings.hotkeys.clone();
    settings.hotkeys = accepted.clone();
    if let Err(error) = persist_settings(&state.settings_path, &settings) {
        settings.hotkeys = previous;
        drop(settings);
        let rollback_error = state.hotkeys.rollback(app, rollback).err();
        return Err(match rollback_error {
            Some(rollback_error) => format!(
                "Could not save hotkeys: {error}. The previous registrations could not be fully restored: {rollback_error}"
            ),
            None => format!("Could not save hotkeys: {error}"),
        });
    }
    Ok(accepted)
}

#[tauri::command]
fn update_hotkeys(
    app: AppHandle,
    hotkeys: HotkeySettings,
    state: State<'_, AppState>,
) -> Result<HotkeySettings, String> {
    apply_hotkeys(&app, state.inner(), hotkeys)
}

#[tauri::command]
fn reset_hotkeys(app: AppHandle, state: State<'_, AppState>) -> Result<HotkeySettings, String> {
    apply_hotkeys(&app, state.inner(), HotkeySettings::default())
}

#[tauri::command]
fn set_hotkey_recording(recording: bool, state: State<'_, AppState>) {
    state.hotkeys.set_recording(recording);
}

#[tauri::command]
fn hide_settings(app: AppHandle) -> Result<(), String> {
    app.state::<AppState>().hotkeys.set_recording(false);
    if let Some(window) = app.get_webview_window("main") {
        window.destroy().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn minimize_settings(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.minimize().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn persist_settings(path: &PathBuf, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(path, json).map_err(|error| error.to_string())
}

fn load_settings(path: &PathBuf) -> AppSettings {
    fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<AppSettings>(&json).ok())
        .unwrap_or_default()
        .validated()
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
        .inner_size(1300.0, 1000.0)
        .min_inner_size(900.0, 650.0)
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
                let state = app.state::<AppState>();
                if let Ok(mut settings) = state.settings.lock() {
                    settings.crosshair.enabled = !settings.crosshair.enabled;
                    state.overlay.update(settings.crosshair.clone());
                    let _ = persist_settings(&state.settings_path, &settings);
                }
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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let state = app.state::<AppState>();
                    if let Some(action) = state.hotkeys.handle_event(shortcut, event) {
                        match action {
                            HotkeyAction::CloseApp => quit_app(app),
                            HotkeyAction::ShowSettings => {
                                let _ = show_settings(app);
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            let settings_path = app
                .path()
                .app_config_dir()
                .map_err(|error| format!("could not resolve the settings directory: {error}"))?
                .join("settings.json");
            let settings = load_settings(&settings_path);
            let overlay = OverlayController::start(settings.crosshair.clone());
            let hotkeys = HotkeyController::new(settings.hotkeys.clone());
            app.manage(AppState {
                settings: Mutex::new(settings),
                settings_path,
                overlay,
                hotkeys,
                quitting: AtomicBool::new(false),
            });
            app.state::<AppState>()
                .hotkeys
                .register_startup(app.handle());
            setup_tray(app.handle())
                .map_err(|error| format!("could not create the system tray icon: {error}"))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
            reset_settings,
            update_hotkeys,
            reset_hotkeys,
            set_hotkey_recording,
            hide_settings,
            minimize_settings
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    window.state::<AppState>().hotkeys.set_recording(false);
                    let _ = window.destroy();
                }
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
