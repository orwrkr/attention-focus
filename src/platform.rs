//! Windows profile storage and process ownership.

use std::{io, path::PathBuf};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE},
    System::{Com::CoTaskMemFree, Threading::CreateMutexW},
    UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
};

pub fn data_dir() -> io::Result<PathBuf> {
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
    pub fn acquire() -> io::Result<Self> {
        // Include the profile in the name so independent Windows users do not collide.
        let name: Vec<u16> = format!("Local\\AttentionFocus-{}", data_dir()?.display())
            .replace(['\\', '/'], "_")
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
            return Err(io::Error::other(
                "Attention Focus is already running. Use Show Focus in its tray menu.",
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
