use crate::settings::CrosshairSettings;
use std::sync::{Arc, OnceLock, RwLock};

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::rasterizer::{OVERLAY_SIZE, rasterize};
    use std::{
        ffi::c_void,
        mem, ptr,
        sync::atomic::{AtomicIsize, Ordering},
        thread,
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
            CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
            HGDIOBJ, ReleaseDC, SelectObject,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics,
                HWND_TOPMOST, MSG, PostMessageW, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE,
                SW_SHOWNOACTIVATE, ShowWindow, TranslateMessage, ULW_ALPHA, UpdateLayeredWindow,
                WM_APP, WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_SETTINGCHANGE, WNDCLASSW,
                WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
                WS_EX_TRANSPARENT, WS_POPUP,
            },
        },
    };

    const WM_REDRAW_OVERLAY: u32 = WM_APP + 41;
    static SETTINGS: OnceLock<Arc<RwLock<CrosshairSettings>>> = OnceLock::new();
    static OVERLAY_HWND: AtomicIsize = AtomicIsize::new(0);

    #[derive(Clone)]
    pub struct OverlayController {
        settings: Arc<RwLock<CrosshairSettings>>,
    }

    impl OverlayController {
        pub fn start(initial: CrosshairSettings) -> Self {
            let settings = Arc::new(RwLock::new(initial));
            let _ = SETTINGS.set(settings.clone());
            thread::Builder::new()
                .name("periscope-overlay".into())
                .spawn(run_message_loop)
                .expect("failed to start overlay thread");
            Self { settings }
        }

        pub fn update(&self, next: CrosshairSettings) {
            *self.settings.write().expect("settings lock poisoned") = next;
            let raw = OVERLAY_HWND.load(Ordering::Acquire);
            if raw != 0 {
                unsafe { PostMessageW(raw as HWND, WM_REDRAW_OVERLAY, 0, 0) };
            }
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_REDRAW_OVERLAY | WM_DISPLAYCHANGE | WM_SETTINGCHANGE | WM_DPICHANGED => {
                unsafe { redraw(hwnd) };
                0
            }
            WM_DESTROY => {
                OVERLAY_HWND.store(0, Ordering::Release);
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    fn run_message_loop() {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let instance = GetModuleHandleW(ptr::null());
            let class_name = wide("periScope.NativeCrosshairOverlay");
            let window_class = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance,
                lpszClassName: class_name.as_ptr(),
                ..mem::zeroed()
            };
            if RegisterClassW(&window_class) == 0 {
                return;
            }

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                wide("periScope crosshair").as_ptr(),
                WS_POPUP,
                0,
                0,
                OVERLAY_SIZE,
                OVERLAY_SIZE,
                ptr::null_mut(),
                ptr::null_mut(),
                instance,
                ptr::null(),
            );
            if hwnd.is_null() {
                return;
            }
            OVERLAY_HWND.store(hwnd as isize, Ordering::Release);
            redraw(hwnd);

            let mut message: MSG = mem::zeroed();
            while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    unsafe fn redraw(hwnd: HWND) {
        let Some(lock) = SETTINGS.get() else { return };
        let settings = lock.read().expect("settings lock poisoned").clone();
        if !settings.enabled {
            unsafe { ShowWindow(hwnd, SW_HIDE) };
            return;
        }

        let mut pixels = vec![0_u32; (OVERLAY_SIZE * OVERLAY_SIZE) as usize];
        rasterize(&mut pixels, &settings);

        unsafe {
            let screen_dc = GetDC(ptr::null_mut());
            if screen_dc.is_null() {
                return;
            }
            let memory_dc = CreateCompatibleDC(screen_dc);
            if memory_dc.is_null() {
                ReleaseDC(ptr::null_mut(), screen_dc);
                return;
            }

            let mut info: BITMAPINFO = mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: OVERLAY_SIZE,
                biHeight: -OVERLAY_SIZE,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..mem::zeroed()
            };
            let mut bitmap_bits: *mut c_void = ptr::null_mut();
            let bitmap = CreateDIBSection(
                screen_dc,
                &info,
                DIB_RGB_COLORS,
                &mut bitmap_bits,
                ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bitmap_bits.is_null() {
                DeleteDC(memory_dc);
                ReleaseDC(ptr::null_mut(), screen_dc);
                return;
            }
            ptr::copy_nonoverlapping(pixels.as_ptr(), bitmap_bits.cast::<u32>(), pixels.len());
            let previous = SelectObject(memory_dc, bitmap as HGDIOBJ);

            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);
            let destination = POINT {
                x: (screen_width - OVERLAY_SIZE) / 2 + settings.x_offset,
                y: (screen_height - OVERLAY_SIZE) / 2 + settings.y_offset,
            };
            let source = POINT { x: 0, y: 0 };
            let size = SIZE {
                cx: OVERLAY_SIZE,
                cy: OVERLAY_SIZE,
            };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                hwnd,
                screen_dc,
                &destination,
                &size,
                memory_dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            );
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);

            SelectObject(memory_dc, previous);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(memory_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
            let _ = HWND_TOPMOST;
        }
    }
}

#[cfg(windows)]
pub use platform::OverlayController;

#[cfg(not(windows))]
#[derive(Clone)]
pub struct OverlayController;

#[cfg(not(windows))]
impl OverlayController {
    pub fn start(_initial: CrosshairSettings) -> Self {
        Self
    }
    pub fn update(&self, _next: CrosshairSettings) {}
}

#[cfg(test)]
mod tests {
    use crate::{
        rasterizer::{OVERLAY_SIZE, rasterize},
        settings::CrosshairSettings,
    };

    fn pixels() -> Vec<u32> {
        vec![0; (OVERLAY_SIZE * OVERLAY_SIZE) as usize]
    }

    fn pixel(pixels: &[u32], x: i32, y: i32) -> u32 {
        pixels[(y * OVERLAY_SIZE + x) as usize]
    }

    #[test]
    fn rasterizes_fill_and_outline_at_exact_geometry_boundaries() {
        let settings = CrosshairSettings {
            center_dot: false,
            ..CrosshairSettings::default()
        };
        let mut output = pixels();
        rasterize(&mut output, &settings);

        assert_eq!(pixel(&output, 113, 128), 0);
        assert_eq!(pixel(&output, 114, 128), 0xff00_0000);
        assert_eq!(pixel(&output, 115, 128), 0xff35_e8ff);
        assert_eq!(pixel(&output, 124, 128), 0xff35_e8ff);
        assert_eq!(pixel(&output, 125, 128), 0xff00_0000);
        assert_eq!(pixel(&output, 126, 128), 0);
    }

    #[test]
    fn t_style_removes_only_the_upper_arm() {
        let settings = CrosshairSettings {
            t_style: true,
            center_dot: false,
            outline: false,
            ..CrosshairSettings::default()
        };
        let mut output = pixels();
        rasterize(&mut output, &settings);

        assert_eq!(pixel(&output, 128, 110), 0);
        assert_eq!(pixel(&output, 128, 140), 0xff35_e8ff);
    }

    #[test]
    fn premultiplies_transparency_and_falls_back_for_malformed_colors() {
        let transparent = CrosshairSettings {
            color: "#FF0000".into(),
            opacity: 50,
            outline: false,
            ..CrosshairSettings::default()
        };
        let mut transparent_output = pixels();
        rasterize(&mut transparent_output, &transparent);
        assert_eq!(pixel(&transparent_output, 128, 128), 0x7f7f_0000);

        let malformed = CrosshairSettings {
            color: "bad".into(),
            outline: false,
            ..CrosshairSettings::default()
        };
        let mut malformed_output = pixels();
        rasterize(&mut malformed_output, &malformed);
        assert_eq!(pixel(&malformed_output, 128, 128), 0xffff_ffff);
    }

    #[test]
    fn clips_rectangles_that_extend_beyond_the_pixel_buffer() {
        let settings = CrosshairSettings {
            length: 200,
            gap: 0,
            thickness: 4,
            center_dot: false,
            outline: false,
            ..CrosshairSettings::default()
        };
        let mut output = pixels();
        rasterize(&mut output, &settings);

        assert_eq!(pixel(&output, 0, 128), 0xff35_e8ff);
        assert_eq!(pixel(&output, 255, 128), 0xff35_e8ff);
    }
}
