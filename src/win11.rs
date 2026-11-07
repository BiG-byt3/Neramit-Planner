//! Windows 11 window chrome for the custom (undecorated) title bar: ask DWM for the rounded
//! corners and thin system border that native windows get when they are not maximized.
//! dwmapi is loaded at run time, so the same exe still starts on Windows 10 (where it's a no-op).

#[cfg(windows)]
pub fn apply(cc: &eframe::CreationContext<'_>) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    extern "system" {
        fn LoadLibraryW(name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }
    type SetAttr = unsafe extern "system" fn(isize, u32, *const c_void, u32) -> i32;
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWA_BORDER_COLOR: u32 = 34;
    const DWMWCP_ROUND: u32 = 2;
    const BORDER: u32 = 0x00C8C8C8; // COLORREF 0x00BBGGRR, Office's light window frame

    let Ok(handle) = cc.window_handle() else { return };
    let RawWindowHandle::Win32(h) = handle.as_raw() else { return };
    let hwnd = h.hwnd.get();
    HWND.store(hwnd, std::sync::atomic::Ordering::Relaxed);
    unsafe {
        let name: Vec<u16> = "dwmapi.dll\0".encode_utf16().collect();
        let lib = LoadLibraryW(name.as_ptr());
        if lib.is_null() {
            return;
        }
        let f = GetProcAddress(lib, b"DwmSetWindowAttribute\0".as_ptr());
        if f.is_null() {
            return;
        }
        let set: SetAttr = std::mem::transmute(f);
        // both attributes are ignored (E_INVALIDARG) before Windows 11 build 22000
        set(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &DWMWCP_ROUND as *const u32 as *const c_void, 4);
        set(hwnd, DWMWA_BORDER_COLOR, &BORDER as *const u32 as *const c_void, 4);
    }
}

#[cfg(not(windows))]
pub fn apply(_cc: &eframe::CreationContext<'_>) {}

static HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// When the window is maximized: how far (physical pixels, left/top/right/bottom) the client area
/// extends beyond the monitor's work area (e.g. under the taskbar on some setups). Usually all zero:
/// Windows keeps the off-screen strip in the invisible resize frame.
#[cfg(windows)]
pub fn maximized_insets() -> Option<[f32; 4]> {
    #[repr(C)]
    #[derive(Default)]
    struct RECT {
        l: i32,
        t: i32,
        r: i32,
        b: i32,
    }
    #[repr(C)]
    struct MONITORINFO {
        size: u32,
        monitor: RECT,
        work: RECT,
        flags: u32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn IsZoomed(h: isize) -> i32;
        fn GetClientRect(h: isize, r: *mut RECT) -> i32;
        fn ClientToScreen(h: isize, p: *mut [i32; 2]) -> i32;
        fn MonitorFromWindow(h: isize, flags: u32) -> isize;
        fn GetMonitorInfoW(m: isize, mi: *mut MONITORINFO) -> i32;
    }
    let hwnd = HWND.load(std::sync::atomic::Ordering::Relaxed);
    if hwnd == 0 {
        return None;
    }
    unsafe {
        if IsZoomed(hwnd) == 0 {
            return None;
        }
        // the client area (what we draw into) in screen coordinates
        let mut cr = RECT::default();
        if GetClientRect(hwnd, &mut cr) == 0 {
            return None;
        }
        let mut origin = [0i32, 0i32];
        if ClientToScreen(hwnd, &mut origin) == 0 {
            return None;
        }
        let wr = RECT { l: origin[0], t: origin[1], r: origin[0] + cr.r, b: origin[1] + cr.b };
        let mon = MonitorFromWindow(hwnd, 2);
        let mut mi = MONITORINFO { size: std::mem::size_of::<MONITORINFO>() as u32, monitor: RECT::default(), work: RECT::default(), flags: 0 };
        if mon == 0 || GetMonitorInfoW(mon, &mut mi) == 0 {
            return None;
        }
        let w = &mi.work;
        let ins = [(w.l - wr.l).max(0) as f32, (w.t - wr.t).max(0) as f32, (wr.r - w.r).max(0) as f32, (wr.b - w.b).max(0) as f32];
        (ins.iter().any(|v| *v > 0.0)).then_some(ins)
    }
}

#[cfg(not(windows))]
pub fn maximized_insets() -> Option<[f32; 4]> {
    None
}
