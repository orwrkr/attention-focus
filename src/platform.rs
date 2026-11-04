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

/// Windows display languages in the user's order of preference, as names such as `ru-RU`.
pub fn preferred_languages() -> Vec<String> {
    use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    let (mut count, mut size) = (0_u32, 0_u32);
    // The first call asks for the size of a list of null-terminated names.
    let sized = unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if sized == 0 || size == 0 {
        return Vec::new();
    }
    let mut buffer = vec![0_u16; size as usize];
    // The buffer has the size that was just reported.
    let filled = unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            buffer.as_mut_ptr(),
            &mut size,
        )
    };
    if filled == 0 {
        return Vec::new();
    }
    buffer
        .split(|unit| *unit == 0)
        .filter(|name| !name.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

/// The system setting that separates a single tray click from a double click.
pub fn double_click_time() -> std::time::Duration {
    let milliseconds =
        unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime() };
    std::time::Duration::from_millis(u64::from(milliseconds))
}

/// The active window of this thread, to own a dialog opened from another thread.
pub fn active_window() -> usize {
    unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow() as usize }
}

/// Report a failure that happens before the window exists.
pub fn show_error_box(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (message, title) = (wide(message), wide("Attention Focus"));
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub struct Instance(HANDLE);

/// Named object within this Windows session, unique to the user and runtime profile.
fn profile_object(kind: &str, directory: &Path) -> io::Result<Vec<u16>> {
    // The resolved directory distinguishes both the Windows user and runtime profile.
    let identity = crate::paths::resolve(directory)?
        .to_string_lossy()
        .to_lowercase()
        .replace(['\\', '/'], "_");
    Ok(format!("Local\\AttentionFocus{kind}-{identity}")
        .encode_utf16()
        .chain(Some(0))
        .collect())
}

/// Ask this profile's running instance to show its window. False if none is listening,
/// which also covers an instance that is still starting up.
pub fn request_show(directory: &Path) -> bool {
    use windows_sys::Win32::{
        System::Threading::{EVENT_MODIFY_STATE, OpenEventW, SetEvent},
        UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow},
    };
    let Ok(name) = profile_object("Show", directory) else {
        return false;
    };
    unsafe {
        let event = OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr());
        if event.is_null() {
            return false;
        }
        // The user just launched this process, so it may pass on the right to come forward.
        AllowSetForegroundWindow(ASFW_ANY);
        let signaled = SetEvent(event) != 0;
        CloseHandle(event);
        signaled
    }
}

/// Receives requests from later launches of the same profile until dropped.
pub struct ShowRequests {
    stop: HANDLE,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ShowRequests {
    pub fn start(directory: &Path, notify: impl Fn() + Send + 'static) -> io::Result<Self> {
        use windows_sys::Win32::{
            Foundation::WAIT_OBJECT_0,
            System::Threading::{CreateEventW, INFINITE, WaitForMultipleObjects},
        };
        let name = profile_object("Show", directory)?;
        let (show, stop) = unsafe {
            (
                CreateEventW(std::ptr::null(), 0, 0, name.as_ptr()),
                CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()),
            )
        };
        if show.is_null() || stop.is_null() {
            let error = io::Error::last_os_error();
            unsafe {
                if !show.is_null() {
                    CloseHandle(show);
                }
                if !stop.is_null() {
                    CloseHandle(stop);
                }
            }
            return Err(error);
        }
        // Kernel handles are safe to wait on from another thread; pass them as integers.
        let handles = [show as usize, stop as usize];
        let thread = std::thread::spawn(move || {
            let waits = handles.map(|handle| handle as HANDLE);
            while unsafe { WaitForMultipleObjects(2, waits.as_ptr(), 0, INFINITE) } == WAIT_OBJECT_0
            {
                notify();
            }
            unsafe {
                CloseHandle(waits[0]);
            }
        });
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for ShowRequests {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Threading::SetEvent(self.stop);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        unsafe {
            CloseHandle(self.stop);
        }
    }
}

impl Instance {
    pub fn acquire(directory: &Path, title: &str) -> io::Result<Self> {
        let name = profile_object("", directory)?;
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{title} is already running. Use its tray menu."),
            ));
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

/// Raise the shown root window above ordinary windows and request activation.
/// Returns true only once Windows reports it as the foreground window, so callers can
/// retry: tray clicks reach here after the double-click delay, when foreground rights
/// may have lapsed, and the viewport commands that restore the window may still be queued.
pub fn bring_to_front(window: &impl raw_window_handle::HasWindowHandle, stay_on_top: bool) -> bool {
    let Ok(handle) = window.window_handle() else {
        return false;
    };
    let raw_window_handle::RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return false;
    };
    unsafe { bring_hwnd_to_front(handle.hwnd.get() as _, stay_on_top) }
}

unsafe fn bring_hwnd_to_front(
    hwnd: windows_sys::Win32::Foundation::HWND,
    stay_on_top: bool,
) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, IsIconic, IsWindowVisible,
    };
    unsafe {
        if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
            return false;
        }
        if GetForegroundWindow() == hwnd {
            return true;
        }
        raise_z_order(hwnd, stay_on_top);
        activate(hwnd)
    }
}

/// Changing Z-order needs no foreground rights, so the window ends up above ordinary
/// windows even when Windows refuses activation. Stay on top keeps its topmost state.
unsafe fn raise_z_order(hwnd: windows_sys::Win32::Foundation::HWND, stay_on_top: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
        SetWindowPos,
    };
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER;
    unsafe {
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);
        if !stay_on_top {
            SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, flags);
        }
    }
}

/// Without foreground rights, share the foreground thread's input state just long enough
/// for one request. This fails for elevated foreground windows, which keep their focus.
/// A hung foreground window is left alone, as sharing its input could stall this thread.
unsafe fn activate(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    use windows_sys::Win32::{
        System::Threading::{AttachThreadInput, GetCurrentThreadId},
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, IsHungAppWindow, SetForegroundWindow,
        },
    };
    unsafe {
        if SetForegroundWindow(hwnd) != 0 && GetForegroundWindow() == hwnd {
            return true;
        }
        let foreground = GetForegroundWindow();
        let thread = if foreground.is_null() || IsHungAppWindow(foreground) != 0 {
            0
        } else {
            GetWindowThreadProcessId(foreground, std::ptr::null_mut())
        };
        let current = GetCurrentThreadId();
        let attached =
            thread != 0 && thread != current && AttachThreadInput(current, thread, 1) != 0;
        SetForegroundWindow(hwnd);
        if attached {
            AttachThreadInput(current, thread, 0);
        }
        GetForegroundWindow() == hwnd
    }
}

/// Repair collapsed restored geometry and constrain restored maximization once.
/// Install on the root window's owning UI thread before its first rendered frame.
pub fn constrain_window_bounds(
    window: &impl raw_window_handle::HasWindowHandle,
    scale_factor: f64,
    center_default: bool,
    drag_regions: &HeaderDragRegions,
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
    // UI-thread-owned rectangles are released when the native window is destroyed.
    let regions = Box::into_raw(Box::new(drag_regions.clone()));
    if unsafe { SetWindowSubclass(hwnd, Some(width_limit), 0xAF18, regions as usize) } == 0 {
        unsafe {
            drop(Box::from_raw(regions));
        }
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
    let monitor = unsafe { monitor_frame(hwnd) };
    let recenter = center_default
        || repaired != current
        || monitor.as_ref().is_some_and(|(m, x, y)| {
            let work = m.rcWork;
            outer.top + y < work.top
                || outer.top + y >= work.bottom
                || outer.right - x <= work.left
                || outer.left + x >= work.right
                || (outer.left <= work.left && outer.top <= work.top)
        });
    if repaired != current || recenter {
        // Repair legacy collapsed geometry once, without clearing any saved preferences.
        let width = outer.right - outer.left - current[0] + repaired[0];
        let height = outer.bottom - outer.top - current[1] + repaired[1];
        let position = if recenter {
            monitor
                .as_ref()
                .map(|(monitor, _, _)| centered_position(&monitor.rcWork, width, height))
        } else {
            None
        };
        if unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                position.map_or(0, |p| p[0]),
                position.map_or(0, |p| p[1]),
                width,
                height,
                if position.is_some() { 0 } else { SWP_NOMOVE } | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Shared only by the root window and its subclass on the UI thread.
#[derive(Clone, Default)]
pub struct HeaderDragRegions(
    std::rc::Rc<std::cell::RefCell<Vec<windows_sys::Win32::Foundation::RECT>>>,
    std::rc::Rc<std::cell::Cell<bool>>,
    std::rc::Rc<std::cell::Cell<bool>>,
);

impl HeaderDragRegions {
    pub fn suppress_first_show(&self) {
        self.1.set(true);
    }
    /// Reconcile winit's cached visibility after the native first-show suppression.
    pub fn take_startup_hide(&self) -> bool {
        self.2.replace(false)
    }
    pub fn set(&self, regions: &[eframe::egui::Rect], scale: f32) {
        use windows_sys::Win32::Foundation::RECT;
        let mut stored = self.0.borrow_mut();
        stored.clear();
        stored.extend(regions.iter().filter(|r| r.is_positive()).map(|r| RECT {
            left: (r.left() * scale).ceil() as i32,
            top: (r.top() * scale).ceil() as i32,
            right: (r.right() * scale).floor() as i32,
            bottom: (r.bottom() * scale).floor() as i32,
        }));
    }
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
    let width = (monitor.rcWork.right - monitor.rcWork.left + 2 * x).min(max_outer_width.max(1));
    let left =
        monitor.rcWork.left + (monitor.rcWork.right - monitor.rcWork.left + 2 * x - width) / 2 - x;
    let top = monitor.rcWork.top - y;
    windows_sys::Win32::Foundation::RECT {
        left,
        top,
        right: left + width,
        bottom: monitor.rcWork.bottom + y,
    }
}

fn centered_position(
    work: &windows_sys::Win32::Foundation::RECT,
    width: i32,
    height: i32,
) -> [i32; 2] {
    [
        work.left + ((work.right - work.left - width) / 2).max(0),
        work.top + ((work.bottom - work.top - height) / 2).max(0),
    ]
}

unsafe extern "system" fn width_limit(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    id: usize,
    data: usize,
) -> isize {
    use windows_sys::Win32::UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass},
        WindowsAndMessaging::{MINMAXINFO, WM_GETMINMAXINFO, WM_NCDESTROY},
    };
    if message == WM_NCDESTROY {
        unsafe { RemoveWindowSubclass(hwnd, Some(width_limit), id) };
        if data != 0 {
            unsafe {
                drop(Box::from_raw(data as *mut HeaderDragRegions));
            }
        }
    }
    // Let winit calculate its DPI-aware frame/drag limits before adjusting maximize.
    let result = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
    if message == windows_sys::Win32::UI::WindowsAndMessaging::WM_WINDOWPOSCHANGING
        && lparam != 0
        && data != 0
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SWP_NOACTIVATE, SWP_SHOWWINDOW, WINDOWPOS,
        };
        let position = unsafe { &mut *(lparam as *mut WINDOWPOS) };
        let state = unsafe { &*(data as *const HeaderDragRegions) };
        if position.flags & SWP_SHOWWINDOW != 0 && state.1.replace(false) {
            position.flags = (position.flags & !SWP_SHOWWINDOW) | SWP_NOACTIVATE;
            state.2.set(true);
        }
    }
    if message == windows_sys::Win32::UI::WindowsAndMessaging::WM_NCHITTEST
        && result == windows_sys::Win32::UI::WindowsAndMessaging::HTCLIENT as isize
        && data != 0
    {
        let mut point = windows_sys::Win32::Foundation::POINT {
            x: lparam as i16 as i32,
            y: (lparam >> 16) as i16 as i32,
        };
        if unsafe { windows_sys::Win32::Graphics::Gdi::ScreenToClient(hwnd, &mut point) } != 0 {
            let regions = unsafe { &*(data as *const HeaderDragRegions) }.0.borrow();
            if regions.iter().any(|r| {
                point.x >= r.left && point.x < r.right && point.y >= r.top && point.y < r.bottom
            }) {
                return windows_sys::Win32::UI::WindowsAndMessaging::HTCAPTION as isize;
            }
        }
    }
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
    fn a_second_launch_reaches_only_the_listening_instance_of_its_profile() {
        let root = tempfile::tempdir().unwrap();
        let (profile, other) = (root.path().join("Production"), root.path().join("Other"));
        assert!(!request_show(&profile), "nothing is listening yet");
        let (sender, receiver) = std::sync::mpsc::channel();
        let requests = ShowRequests::start(&profile, move || {
            let _ = sender.send(());
        })
        .unwrap();
        assert!(!request_show(&other), "profiles stay independent");
        assert!(request_show(&profile));
        let timeout = std::time::Duration::from_secs(5);
        receiver.recv_timeout(timeout).unwrap();
        assert!(request_show(&profile));
        receiver.recv_timeout(timeout).unwrap();
        drop(requests);
        assert!(!request_show(&profile), "stopping removes the named event");
    }
    #[test]
    fn raising_orders_above_ordinary_windows_and_preserves_stay_on_top() {
        use windows_sys::Win32::{
            Foundation::HWND, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*,
        };
        unsafe extern "system" fn procedure(
            hwnd: HWND,
            message: u32,
            wparam: usize,
            lparam: isize,
        ) -> isize {
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        fn above(upper: HWND, lower: HWND) -> bool {
            let mut current = lower;
            while !current.is_null() {
                current = unsafe { GetWindow(current, GW_HWNDPREV) };
                if current == upper {
                    return true;
                }
            }
            false
        }
        let topmost =
            |window| unsafe { GetWindowLongPtrW(window, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST != 0 };
        let name: Vec<u16> = "AttentionFocusRaiseTest"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class = WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance,
                lpszClassName: name.as_ptr(),
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&class), 0);
            // Off-screen and never activated: this test must not disturb the desktop.
            let create = || {
                let window = CreateWindowExW(
                    WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                    name.as_ptr(),
                    name.as_ptr(),
                    WS_POPUP,
                    -32_000,
                    -32_000,
                    16,
                    16,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    instance,
                    std::ptr::null(),
                );
                assert!(!window.is_null());
                window
            };
            let (first, second) = (create(), create());
            // A hidden window is not shown or activated before its commands are applied.
            assert!(!bring_hwnd_to_front(first, false));
            assert_eq!(IsWindowVisible(first), 0);
            ShowWindow(first, SW_SHOWNOACTIVATE);
            ShowWindow(second, SW_SHOWNOACTIVATE);
            raise_z_order(second, false);
            assert!(above(second, first));
            raise_z_order(first, false);
            assert!(above(first, second));
            assert!(!topmost(first));
            raise_z_order(second, true);
            assert!(above(second, first));
            assert!(topmost(second));
            raise_z_order(second, true);
            assert!(topmost(second), "Stay on top must survive repeated raises");
            raise_z_order(second, false);
            assert!(!topmost(second));
            assert_ne!(DestroyWindow(first), 0);
            assert_ne!(DestroyWindow(second), 0);
            assert_ne!(UnregisterClassW(name.as_ptr(), instance), 0);
        }
    }
    #[test]
    fn centering_uses_work_area_including_negative_monitor_coordinates() {
        use windows_sys::Win32::Foundation::RECT;
        let work = RECT {
            left: -1920,
            top: 48,
            right: 0,
            bottom: 1080,
        };
        assert_eq!(centered_position(&work, 620, 640), [-1270, 244]);
        assert_eq!(centered_position(&work, 2200, 1200), [-1920, 48]);
        let monitor = windows_sys::Win32::Graphics::Gdi::MONITORINFO {
            rcWork: work,
            ..Default::default()
        };
        let bounds = maximized_bounds(&monitor, 8, 8, 776);
        assert_eq!(bounds.left + bounds.right, work.left + work.right);
        assert_eq!(bounds.top + 8, work.top);
    }
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
            if message == WM_NCHITTEST {
                return HTCLIENT as isize;
            }
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
                    + (monitor.rcWork.right - monitor.rcWork.left + 2 * x - 800) / 2
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
            let header = HeaderDragRegions::default();
            header
                .0
                .borrow_mut()
                .push(windows_sys::Win32::Foundation::RECT {
                    left: 100,
                    top: 5,
                    right: 190,
                    bottom: 31,
                });
            let regions = Box::into_raw(Box::new(header));
            assert_ne!(
                SetWindowSubclass(window, Some(width_limit), 0xAF18, regions as usize),
                0
            );
            for (x, expected) in [(150, HTCAPTION), (30, HTCLIENT)] {
                let mut point = windows_sys::Win32::Foundation::POINT { x, y: 18 };
                assert_ne!(
                    windows_sys::Win32::Graphics::Gdi::ClientToScreen(window, &mut point),
                    0
                );
                let coordinates = ((point.y as u32 & 0xffff) << 16) | (point.x as u32 & 0xffff);
                assert_eq!(
                    SendMessageW(window, WM_NCHITTEST, 0, coordinates as isize),
                    expected as isize
                );
            }
            let header = &*regions;
            header.suppress_first_show();
            ShowWindow(window, SW_SHOW);
            assert_eq!(
                IsWindowVisible(window),
                0,
                "first autostart show must remain hidden"
            );
            assert!(header.take_startup_hide());
            assert!(!header.take_startup_hide());
            ShowWindow(window, SW_SHOW);
            assert_ne!(IsWindowVisible(window), 0, "later explicit Show must work");
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
