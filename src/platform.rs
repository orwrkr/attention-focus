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

#[cfg(test)]
mod tests {
    use super::*;
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
