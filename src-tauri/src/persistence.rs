use crate::settings::{AppSettings, LegacyAppSettings};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) fn persist_settings(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let temporary = temporary_path(path);
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        let json = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
        file.write_all(&json).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        replace_file(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn load_settings(path: &Path) -> AppSettings {
    fs::read_to_string(path)
        .ok()
        .and_then(|json| {
            let mut value: serde_json::Value = serde_json::from_str(&json).ok()?;
            if value
                .get("presets")
                .is_some_and(serde_json::Value::is_array)
            {
                upgrade_default_dot(&mut value);
                serde_json::from_value::<AppSettings>(value).ok()
            } else {
                serde_json::from_value::<LegacyAppSettings>(value)
                    .ok()
                    .map(LegacyAppSettings::migrate)
            }
        })
        .unwrap_or_default()
        .validated()
}

fn upgrade_default_dot(value: &mut serde_json::Value) {
    let Some(presets) = value
        .get_mut("presets")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    for preset in presets {
        if preset.get("id").and_then(serde_json::Value::as_str) != Some("dot") {
            continue;
        }
        let Some(settings) = preset
            .get_mut("settings")
            .and_then(serde_json::Value::as_object_mut)
        else {
            continue;
        };
        if settings.contains_key("dotOnly") {
            continue;
        }
        if settings.get("length").and_then(serde_json::Value::as_i64) == Some(1)
            && settings.get("gap").and_then(serde_json::Value::as_i64) == Some(0)
        {
            settings.insert("dotOnly".into(), true.into());
            settings.insert("length".into(), 10.into());
            settings.insert("gap".into(), 3.into());
        }
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings");
    path.with_file_name(format!(".{name}.{}.tmp", std::process::id()))
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temporary = temporary
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            temporary.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(temporary, destination).map_err(|error| error.to_string())
}
