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

/// Apply the configured tracking-width limit to maximization as well as dragging.
/// Install on the root window's owning UI thread before its first rendered frame.
pub fn constrain_maximized_width(
    window: &impl raw_window_handle::HasWindowHandle,
    max_client_width: u32,
) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::{
            Shell::SetWindowSubclass,
            WindowsAndMessaging::{
                GetClientRect, GetWindowRect, IsZoomed, MINMAXINFO, SWP_NOACTIVATE, SWP_NOMOVE,
                SWP_NOZORDER, SendMessageW, SetWindowPos, WM_GETMINMAXINFO,
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
    if unsafe { IsZoomed(hwnd) } != 0
        && let Some((monitor, _, _)) = unsafe { monitor_frame(hwnd) }
    {
        let mut limits = MINMAXINFO::default();
        unsafe {
            SendMessageW(
                hwnd,
                WM_GETMINMAXINFO,
                0,
                (&mut limits as *mut MINMAXINFO) as isize,
            )
        };
        if unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                monitor.rcMonitor.left + limits.ptMaxPosition.x,
                monitor.rcMonitor.top + limits.ptMaxPosition.y,
                limits.ptMaxSize.x,
                limits.ptMaxSize.y,
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
    let client_width = client.right - client.left;
    if client_width > max_client_width as i32 {
        // Correct restored oversized geometry once, while eframe still keeps it hidden.
        // Keep position, height, maximized state and activation unchanged.
        let width = outer.right - outer.left - client_width + max_client_width as i32;
        if unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                width,
                outer.bottom - outer.top,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
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
            // Include the invisible resize borders, so the caption/client start inside
            // the work area instead of being clipped above it at non-default DPI.
            limits.ptMaxPosition.x = monitor.rcWork.left - monitor.rcMonitor.left - x;
            limits.ptMaxPosition.y = monitor.rcWork.top - monitor.rcMonitor.top - y;
            limits.ptMaxSize.x =
                (monitor.rcWork.right - monitor.rcWork.left + 2 * x).min(limits.ptMaxTrackSize.x);
            limits.ptMaxSize.y = monitor.rcWork.bottom - monitor.rcWork.top + 2 * y;
        }
    }
    result
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
