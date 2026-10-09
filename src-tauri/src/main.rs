#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
struct InstanceGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(windows)]
fn acquire_instance() -> Option<InstanceGuard> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError},
        System::Threading::CreateMutexW,
    };

    let name = "Local\\app.periscope.crosshair.SingleInstance"
        .encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        panic!(
            "Could not check for a running periScope instance: {}",
            std::io::Error::last_os_error()
        );
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe { CloseHandle(handle) };
        return None;
    }
    Some(InstanceGuard(handle))
}

fn main() {
    #[cfg(windows)]
    let Some(_instance_guard) = acquire_instance() else {
        return;
    };
    periscope_lib::run();
}
