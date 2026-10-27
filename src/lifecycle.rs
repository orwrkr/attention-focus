//! Hidden Windows notification window, independent of the visible egui window.
use std::{io, sync::mpsc};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::{
        LibraryLoader::GetModuleHandleW,
        RemoteDesktop::{
            NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
            WTSUnRegisterSessionNotification,
        },
    },
    UI::WindowsAndMessaging::*,
};
type Notify = Box<dyn Fn(&'static str) + Send>;

pub struct Listener {
    window: usize,
}
impl Listener {
    pub fn start(notify: impl Fn(&'static str) + Send + 'static) -> io::Result<Self> {
        let (ready, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let class: Vec<u16> = "AttentionFocusLifecycle"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
            let descriptor = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: instance,
                lpszClassName: class.as_ptr(),
                ..Default::default()
            };
            if unsafe { RegisterClassW(&descriptor) } == 0 {
                let _ = ready.send(Err(io::Error::last_os_error()));
                return;
            }
            let mut notify: Notify = Box::new(notify);
            // This box stays alive on the message-loop thread until its window is destroyed.
            let window = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    class.as_ptr(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    instance,
                    (&mut notify as *mut Notify).cast(),
                )
            };
            if window.is_null()
                || unsafe { WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION) } == 0
            {
                let error = io::Error::last_os_error();
                if !window.is_null() {
                    unsafe {
                        DestroyWindow(window);
                    }
                }
                unsafe {
                    UnregisterClassW(class.as_ptr(), instance);
                }
                let _ = ready.send(Err(error));
                return;
            }
            let _ = ready.send(Ok(window as usize));
            let mut message = MSG::default();
            while unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } > 0 {
                unsafe {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            unsafe {
                if IsWindow(window) != 0 {
                    WTSUnRegisterSessionNotification(window);
                    DestroyWindow(window);
                }
                UnregisterClassW(class.as_ptr(), instance);
            }
        });
        let window = receiver
            .recv()
            .map_err(|_| io::Error::other("Lifecycle listener initialization failed."))??;
        Ok(Self { window })
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        unsafe {
            PostMessageW(self.window as HWND, WM_CLOSE, 0, 0);
        }
    }
}
unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let creation = unsafe { &*(lparam as *const CREATESTRUCTW) };
        unsafe {
            SetWindowLongPtrW(window, GWLP_USERDATA, creation.lpCreateParams as isize);
        }
    }
    let pointer = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *const Notify;
    let reason = match message {
        WM_WTSSESSION_CHANGE if wparam == WTS_SESSION_LOCK as usize => Some("lock"),
        WM_POWERBROADCAST if wparam == PBT_APMSUSPEND as usize => Some("suspend"),
        WM_ENDSESSION if wparam != 0 => Some(if lparam & ENDSESSION_LOGOFF as isize != 0 {
            "logoff"
        } else {
            "shutdown"
        }),
        _ => None,
    };
    if let Some(reason) = reason
        && !pointer.is_null()
    {
        unsafe {
            (&*pointer)(reason);
        }
    }
    match message {
        // Agree immediately; save only when Windows confirms the session will end.
        WM_QUERYENDSESSION => 1,
        WM_ENDSESSION => 0,
        WM_CLOSE => {
            unsafe {
                WTSUnRegisterSessionNotification(window);
                DestroyWindow(window);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        WM_POWERBROADCAST => 1,
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notification_window_routes_confirmed_interruptions_but_ignores_resume_and_cancellation() {
        let (sender, receiver) = mpsc::channel();
        let listener = Listener::start(move |reason| {
            let _ = sender.send(reason);
        })
        .unwrap();
        unsafe {
            PostMessageW(
                listener.window as HWND,
                WM_WTSSESSION_CHANGE,
                WTS_SESSION_LOCK as usize,
                0,
            );
        }
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap(),
            "lock"
        );
        unsafe {
            PostMessageW(
                listener.window as HWND,
                WM_WTSSESSION_CHANGE,
                WTS_SESSION_UNLOCK as usize,
                0,
            );
            PostMessageW(
                listener.window as HWND,
                WM_POWERBROADCAST,
                PBT_APMRESUMEAUTOMATIC as usize,
                0,
            );
            PostMessageW(
                listener.window as HWND,
                WM_POWERBROADCAST,
                PBT_APMSUSPEND as usize,
                0,
            );
        }
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap(),
            "suspend"
        );
        assert!(receiver.try_recv().is_err());
        let window = listener.window as HWND;
        unsafe {
            assert_eq!(
                SendMessageW(window, WM_QUERYENDSESSION, 0, ENDSESSION_LOGOFF as isize),
                1
            );
            SendMessageW(window, WM_ENDSESSION, 0, ENDSESSION_LOGOFF as isize);
        }
        assert!(receiver.try_recv().is_err());
        unsafe {
            SendMessageW(
                window,
                WM_ENDSESSION,
                1,
                (ENDSESSION_LOGOFF | ENDSESSION_CRITICAL) as isize,
            );
            SendMessageW(window, WM_ENDSESSION, 1, 0);
        }
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap(),
            "logoff"
        );
        assert_eq!(
            receiver
                .recv_timeout(std::time::Duration::from_secs(3))
                .unwrap(),
            "shutdown"
        );
        assert!(receiver.try_recv().is_err());
    }
}
