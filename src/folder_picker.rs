//! Existing-directory selection on an isolated STA thread; this module writes no files.
use std::{
    io,
    path::{Path, PathBuf},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM},
    System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoTaskMemFree,
        CoUninitialize,
    },
    UI::{Shell::*, WindowsAndMessaging::SendMessageW},
};

unsafe extern "system" fn initialize(
    hwnd: HWND,
    message: u32,
    _value: LPARAM,
    initial: LPARAM,
) -> i32 {
    if message == BFFM_INITIALIZED && initial != 0 {
        unsafe { SendMessageW(hwnd, BFFM_SETSELECTIONW, 1, initial) };
    }
    0
}

pub fn pick(owner: usize, initial: &Path) -> io::Result<Option<PathBuf>> {
    let status = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    if status < 0 {
        return Err(io::Error::other(format!(
            "Cannot initialize folder selection: {status:#x}"
        )));
    }
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _apartment = Apartment;
    let title: Vec<u16> = "Choose the MetaPlan Instance folder (read-only)"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let text = initial.to_string_lossy();
    let shell_path = if let Some(path) = text.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{path}")
    } else {
        text.strip_prefix("\\\\?\\").unwrap_or(&text).to_owned()
    };
    let initial: Vec<u16> = shell_path.encode_utf16().chain(Some(0)).collect();
    let mut display = vec![0u16; 32768];
    let info = BROWSEINFOW {
        hwndOwner: owner as _,
        pszDisplayName: display.as_mut_ptr(),
        lpszTitle: title.as_ptr(),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | BIF_EDITBOX | BIF_NONEWFOLDERBUTTON,
        lpfn: Some(initialize),
        lParam: initial.as_ptr() as isize,
        ..Default::default()
    };
    let item = unsafe { SHBrowseForFolderW(&info) };
    if item.is_null() {
        return Ok(None);
    }
    let mut path = vec![0u16; 32768];
    let ok = unsafe {
        SHGetPathFromIDListEx(item, path.as_mut_ptr(), path.len() as u32, GPFIDL_DEFAULT)
    };
    unsafe { CoTaskMemFree(item.cast()) };
    if ok == 0 {
        return Err(io::Error::other("Choose an existing filesystem folder."));
    }
    let length = path.iter().position(|&c| c == 0).unwrap_or(path.len());
    use std::os::windows::ffi::OsStringExt;
    Ok(Some(std::ffi::OsString::from_wide(&path[..length]).into()))
}
