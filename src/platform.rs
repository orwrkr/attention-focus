//! Windows profile storage and process ownership.

use std::{
    io,
    path::{Path, PathBuf},
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
    System::{Com::CoTaskMemFree, Threading::CreateMutexW},
    UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
};

pub fn data_root() -> io::Result<PathBuf> {
    let mut pointer = std::ptr::null_mut();
    // The shell allocates a null-terminated UTF-16 buffer; release it with COM.
    let status = unsafe {
        SHGetKnownFolderPath(
            &FOLDERID_LocalAppData,
            0,
            std::ptr::null_mut(),
            &mut pointer,
        )
    };
    if status < 0 {
        return Err(io::Error::other(format!(
            "LocalAppData lookup failed: {status:#x}"
        )));
    }
    let mut length = 0;
    unsafe {
        while *pointer.add(length) != 0 {
            length += 1;
        }
    }
    use std::os::windows::ffi::OsStringExt;
    let path =
        std::ffi::OsString::from_wide(unsafe { std::slice::from_raw_parts(pointer, length) });
    unsafe {
        CoTaskMemFree(pointer.cast());
    }
    Ok(PathBuf::from(path).join("AttentionFocus"))
}

pub struct Instance(HANDLE);

impl Instance {
    pub fn acquire(directory: &Path, title: &str) -> io::Result<Self> {
        // The resolved directory distinguishes both the Windows user and runtime profile.
        let identity = crate::source::resolve(directory)?
            .to_string_lossy()
            .to_lowercase()
            .replace(['\\', '/'], "_");
        let name: Vec<u16> = format!("Local\\AttentionFocus-{identity}")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            return Err(io::Error::other(format!(
                "{title} is already running. Use its tray menu."
            )));
        }
        Ok(Self(handle))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Repair collapsed restored geometry and constrain restored maximization once.
/// Install on the root window's owning UI thread before its first rendered frame.
pub fn constrain_window_bounds(
    window: &impl raw_window_handle::HasWindowHandle,
    scale_factor: f64,
) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::{
            Shell::SetWindowSubclass,
            WindowsAndMessaging::{
                GetClientRect, GetWindowRect, IsZoomed, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER,
                SetWindowPos,
            },
        },
    };
    let raw_window_handle::RawWindowHandle::Win32(handle) =
        window.window_handle().map_err(io::Error::other)?.as_raw()
    else {
        return Err(io::Error::other("Expected a Windows root window."));
    };
    let hwnd = handle.hwnd.get() as _;
    // The static callback owns no Rust data and removes itself on native destruction.
    if unsafe { SetWindowSubclass(hwnd, Some(width_limit), 0xAF18, 0) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let max_client_width = (760.0 * scale_factor).round() as i32;
    if unsafe { IsZoomed(hwnd) } != 0
        && let Some((monitor, x, y)) = unsafe { monitor_frame(hwnd) }
    {
        // Do not synthesize WM_GETMINMAXINFO: Windows normally seeds its structure,
        // while winit may not yet have installed its min/max constraints here.
        let bounds = maximized_bounds(&monitor, x, y, max_client_width + 2 * x);
        if unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        return Ok(());
    }
    let (mut outer, mut client) = (RECT::default(), RECT::default());
    if unsafe { GetWindowRect(hwnd, &mut outer) } == 0
        || unsafe { GetClientRect(hwnd, &mut client) } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let current = [client.right - client.left, client.bottom - client.top];
    let repaired = restored_client_size(current, scale_factor);
    if repaired != current {
        // Repair legacy collapsed geometry once, without clearing any saved preferences.
        let width = outer.right - outer.left - current[0] + repaired[0];
        let height = outer.bottom - outer.top - current[1] + repaired[1];
        if unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                width,
                height,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

fn restored_client_size(current: [i32; 2], scale: f64) -> [i32; 2] {
    let min = [420.0, 360.0].map(|v| (v * scale).round() as i32);
    let fallback = [620.0, 640.0].map(|v| (v * scale).round() as i32);
    let max = (760.0 * scale).round() as i32;
    [
        if current[0] < min[0] {
            fallback[0]
        } else {
            current[0].min(max)
        },
        if current[1] < min[1] {
            fallback[1]
        } else {
            current[1]
        },
    ]
}

fn maximized_bounds(
    monitor: &windows_sys::Win32::Graphics::Gdi::MONITORINFO,
    x: i32,
    y: i32,
    max_outer_width: i32,
) -> windows_sys::Win32::Foundation::RECT {
    let left = monitor.rcWork.left - x;
    let top = monitor.rcWork.top - y;
    windows_sys::Win32::Foundation::RECT {
        left,
        top,
        right: left
            + (monitor.rcWork.right - monitor.rcWork.left + 2 * x).min(max_outer_width.max(1)),
        bottom: monitor.rcWork.bottom + y,
    }
}

unsafe extern "system" fn width_limit(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    id: usize,
    _data: usize,
) -> isize {
    use windows_sys::Win32::UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass},
        WindowsAndMessaging::{MINMAXINFO, WM_GETMINMAXINFO, WM_NCDESTROY},
    };
    if message == WM_NCDESTROY {
        unsafe { RemoveWindowSubclass(hwnd, Some(width_limit), id) };
    }
    // Let winit calculate its DPI-aware frame/drag limits before adjusting maximize.
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if message == WM_GETMINMAXINFO && lparam != 0 {
        let limits = unsafe { &mut *(lparam as *mut MINMAXINFO) };
        if let Some((monitor, x, y)) = unsafe { monitor_frame(hwnd) } {
            // A real Windows message supplies defaults, but tolerate unset tracking
            // limits during startup. Never let a zero upper bound collapse the window.
            let fallback = (760.0
                * unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForWindow(hwnd) }.max(96) as f64
                / 96.0)
                .round() as i32
                + 2 * x;
            let cap = if limits.ptMaxTrackSize.x > 0 {
                limits.ptMaxTrackSize.x
            } else {
                fallback
            };
            let bounds = maximized_bounds(&monitor, x, y, cap);
            limits.ptMaxPosition.x = bounds.left - monitor.rcMonitor.left;
            limits.ptMaxPosition.y = bounds.top - monitor.rcMonitor.top;
            limits.ptMaxSize.x = bounds.right - bounds.left;
            limits.ptMaxSize.y = bounds.bottom - bounds.top;
        }
    }
    if matches!(
        message,
        windows_sys::Win32::UI::WindowsAndMessaging::WM_NCPAINT
            | windows_sys::Win32::UI::WindowsAndMessaging::WM_NCACTIVATE
            | windows_sys::Win32::UI::WindowsAndMessaging::WM_SIZE
    ) {
        unsafe { paint_maximized_frame(hwnd) };
    }
    result
}

unsafe fn paint_maximized_frame(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::{Foundation::RECT, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
    if unsafe { IsZoomed(hwnd) } == 0 {
        return;
    }
    let Some((_, x, y)) = (unsafe { monitor_frame(hwnd) }) else {
        return;
    };
    let mut bounds = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut bounds) } == 0 {
        return;
    }
    let visible = RECT {
        left: x,
        top: y,
        right: bounds.right - bounds.left - x,
        bottom: bounds.bottom - bounds.top - y,
    };
    let dc = unsafe { GetWindowDC(hwnd) };
    if dc.is_null() {
        return;
    }
    let brush = unsafe { CreateSolidBrush(135 | (145 << 8) | (155 << 16)) };
    if !brush.is_null() {
        unsafe {
            FrameRect(dc, &visible, brush);
            DeleteObject(brush.cast());
        }
    }
    unsafe { ReleaseDC(hwnd, dc) };
}

unsafe fn monitor_frame(
    hwnd: windows_sys::Win32::Foundation::HWND,
) -> Option<(windows_sys::Win32::Graphics::Gdi::MONITORINFO, i32, i32)> {
    use windows_sys::Win32::{
        Graphics::Gdi::*,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    };
    let mut monitor = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe {
        GetMonitorInfoW(
            MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST),
            &mut monitor,
        )
    } == 0
    {
        return None;
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
    let padding = unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) };
    Some((
        monitor,
        unsafe { GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi) } + padding,
        unsafe { GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) } + padding,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collapsed_restore_recovers_without_changing_valid_dimensions() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let physical = |value: f64| (value * scale).round() as i32;
            assert_eq!(
                restored_client_size([64, 64], scale),
                [physical(620.0), physical(640.0)]
            );
            assert_eq!(
                restored_client_size([physical(500.0), physical(900.0)], scale),
                [physical(500.0), physical(900.0)]
            );
            assert_eq!(
                restored_client_size([physical(500.0), 0], scale),
                [physical(500.0), physical(640.0)]
            );
            assert_eq!(
                restored_client_size([physical(1000.0), physical(900.0)], scale),
                [physical(760.0), physical(900.0)]
            );
        }
    }
    #[test]
    fn maximize_subclass_chains_original_limits_and_accounts_for_monitor_frame() {
        use windows_sys::Win32::{
            Foundation::{HWND, LPARAM, LRESULT, WPARAM},
            System::LibraryLoader::GetModuleHandleW,
            UI::{Shell::SetWindowSubclass, WindowsAndMessaging::*},
        };
        unsafe extern "system" fn original(
            hwnd: HWND,
            message: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if message == WM_GETMINMAXINFO {
                // Match winit startup before min/max constraints are installed:
                // it consumes the message without filling Windows default fields.
                if wparam == 1 {
                    return 0;
                }
                let limits = unsafe { &mut *(lparam as *mut MINMAXINFO) };
                limits.ptMaxSize.x = 2000;
                limits.ptMaxSize.y = 1000;
                limits.ptMaxTrackSize.x = 800;
                limits.ptMaxTrackSize.y = 16000;
                return 0;
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        let name: Vec<u16> = "AttentionFocusWidthTest"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(original),
                hInstance: instance,
                lpszClassName: name.as_ptr(),
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&class), 0);
            let window = CreateWindowExW(
                0,
                name.as_ptr(),
                name.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                200,
                200,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            assert!(!window.is_null());
            assert_ne!(SetWindowSubclass(window, Some(width_limit), 0xAF18, 0), 0);
            let mut limits = MINMAXINFO::default();
            SendMessageW(
                window,
                WM_GETMINMAXINFO,
                0,
                (&mut limits as *mut MINMAXINFO) as isize,
            );
            assert_eq!(limits.ptMaxSize.x, 800);
            let (monitor, x, y) = monitor_frame(window).unwrap();
            assert_eq!(
                limits.ptMaxSize.y,
                monitor.rcWork.bottom - monitor.rcWork.top + 2 * y
            );
            assert_eq!(
                limits.ptMaxPosition.x,
                monitor.rcWork.left - monitor.rcMonitor.left - x
            );
            assert_eq!(
                limits.ptMaxPosition.y,
                monitor.rcWork.top - monitor.rcMonitor.top - y
            );
            assert_eq!(limits.ptMaxTrackSize.y, 16000);
            let mut startup = MINMAXINFO::default();
            SendMessageW(
                window,
                WM_GETMINMAXINFO,
                1,
                (&mut startup as *mut MINMAXINFO) as isize,
            );
            let dpi = windows_sys::Win32::UI::HiDpi::GetDpiForWindow(window).max(96);
            let expected_width = (760.0 * dpi as f64 / 96.0).round() as i32 + 2 * x;
            assert_eq!(
                startup.ptMaxSize.x,
                expected_width.min(monitor.rcWork.right - monitor.rcWork.left + 2 * x)
            );
            assert!(startup.ptMaxSize.y > 0);
            assert_ne!(DestroyWindow(window), 0);
            assert_ne!(UnregisterClassW(name.as_ptr(), instance), 0);
        }
    }
    #[test]
    fn production_and_development_can_run_together_but_duplicates_cannot() {
        let root = tempfile::tempdir().unwrap();
        let prod = root.path().join("Production");
        let dev = root.path().join("Development");
        let production = Instance::acquire(&prod, "Production test").unwrap();
        let development = Instance::acquire(&dev, "Development test").unwrap();
        assert!(Instance::acquire(&prod, "Production test").is_err());
        assert!(Instance::acquire(&dev, "Development test").is_err());
        std::fs::create_dir_all(&prod).unwrap();
        assert!(Instance::acquire(&prod, "Production test").is_err());
        drop(development);
        drop(production);
        assert!(Instance::acquire(&prod, "Production test").is_ok());
    }
}
