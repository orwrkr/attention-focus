//! Explicit per-user sign-in registration. Ordinary startup performs reads only.
use crate::profiles::Profile;
use std::{io, path::Path};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::*,
};

const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

#[derive(Debug, PartialEq, Eq)]
pub struct Launch {
    pub profile: Profile,
    pub autostart: bool,
    pub tray: bool,
}
impl Launch {
    pub fn parse(arguments: &[String], debug: bool) -> io::Result<Self> {
        let mut profile_args = Vec::new();
        let (mut autostart, mut tray) = (false, false);
        for argument in arguments {
            match argument.as_str() {
                "--autostart" if !autostart => autostart = true,
                "--start-in-tray" if !tray => tray = true,
                _ => profile_args.push(argument.clone()),
            }
        }
        if tray && !autostart {
            return Err(io::Error::other("--start-in-tray requires --autostart."));
        }
        Ok(Self {
            profile: Profile::parse(&profile_args, debug)?,
            autostart,
            tray,
        })
    }
    pub fn hide_window(&self, tray_available: bool) -> bool {
        self.autostart && self.tray && tray_available
    }
}

fn name(profile: Profile) -> &'static str {
    match profile {
        Profile::Production => "AttentionFocus",
        Profile::Development => "AttentionFocusDevelopment",
    }
}

pub fn command(executable: &Path, profile: Profile, tray: bool) -> io::Result<String> {
    let path = executable.to_str().ok_or_else(|| {
        io::Error::other(crate::i18n::message("errors.startup_path_unicode", &[]))
    })?;
    let path = if let Some(unc) = path.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{unc}")
    } else {
        path.strip_prefix("\\\\?\\").unwrap_or(path).to_owned()
    };
    if !executable.is_absolute() || path.contains(['"', '\0']) {
        return Err(io::Error::other(crate::i18n::message(
            "errors.startup_path_absolute",
            &[],
        )));
    }
    let command = format!(
        "\"{path}\" --profile {} --autostart{}",
        if profile == Profile::Production {
            "production"
        } else {
            "development"
        },
        if tray { " --start-in-tray" } else { "" }
    );
    if command.encode_utf16().count() > 260 {
        return Err(io::Error::other(crate::i18n::message(
            "errors.startup_command_long",
            &[],
        )));
    }
    Ok(command)
}

pub fn read(profile: Profile) -> io::Result<Option<String>> {
    read_entry(RUN, name(profile))
}
pub fn set(profile: Profile, command: Option<&str>) -> io::Result<()> {
    set_entry(RUN, name(profile), command)
}

struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn checked(status: u32) -> io::Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::other(crate::i18n::message(
            "errors.startup_registry_failed",
            &[("code", &status.to_string())],
        )))
    }
}
fn read_entry(path: &str, name: &str) -> io::Result<Option<String>> {
    let mut key = std::ptr::null_mut();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(path).as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    checked(status)?;
    let key = Key(key);
    let (mut kind, mut bytes) = (0, 0);
    let name = wide(name);
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    checked(status)?;
    if kind != REG_SZ || bytes < 2 || bytes % 2 != 0 || bytes > 65536 {
        return Err(io::Error::other(crate::i18n::message(
            "errors.startup_invalid_format",
            &[],
        )));
    }
    let mut buffer = vec![0_u16; bytes as usize / 2];
    checked(unsafe {
        RegQueryValueExW(
            key.0,
            name.as_ptr(),
            std::ptr::null(),
            &mut kind,
            buffer.as_mut_ptr().cast(),
            &mut bytes,
        )
    })?;
    if kind != REG_SZ || bytes < 2 || bytes % 2 != 0 || bytes as usize > buffer.len() * 2 {
        return Err(io::Error::other(crate::i18n::message(
            "errors.startup_changed_while_reading",
            &[],
        )));
    }
    buffer.truncate(bytes as usize / 2);
    if buffer.pop() != Some(0) || buffer.contains(&0) {
        return Err(io::Error::other(crate::i18n::message(
            "errors.startup_invalid_terminator",
            &[],
        )));
    }
    String::from_utf16(&buffer)
        .map(Some)
        .map_err(|_| io::Error::other(crate::i18n::message("errors.startup_invalid_unicode", &[])))
}
fn set_entry(path: &str, name: &str, value: Option<&str>) -> io::Result<()> {
    let mut key = std::ptr::null_mut();
    let status = if value.is_some() {
        unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(path).as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut key,
                std::ptr::null_mut(),
            )
        }
    } else {
        unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                wide(path).as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut key,
            )
        }
    };
    if value.is_none() && status == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    checked(status)?;
    let key = Key(key);
    if let Some(value) = value {
        let value = wide(value);
        checked(unsafe {
            RegSetValueExW(
                key.0,
                wide(name).as_ptr(),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        })
    } else {
        let status = unsafe { RegDeleteValueW(key.0, wide(name).as_ptr()) };
        if status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            checked(status)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_flags_are_explicit_and_never_cross_debug_profiles() {
        let parse = |args: &[&str], debug| {
            Launch::parse(&args.iter().map(|s| (*s).into()).collect::<Vec<_>>(), debug)
        };
        assert!(!parse(&[], true).unwrap().hide_window(true));
        assert!(!parse(&["--autostart"], true).unwrap().hide_window(true));
        let launch = parse(
            &["--profile", "development", "--autostart", "--start-in-tray"],
            true,
        )
        .unwrap();
        assert!(launch.hide_window(true));
        assert!(!launch.hide_window(false));
        assert!(parse(&["--start-in-tray"], false).is_err());
        assert!(parse(&["--autostart", "--autostart"], false).is_err());
        assert!(parse(&["--profile", "production", "--autostart"], true).is_err());
    }
    #[test]
    fn commands_quote_paths_and_preserve_profile_and_unicode() {
        let path = Path::new("C:\\Example App\\Focus.exe");
        assert_eq!(
            command(path, Profile::Production, true).unwrap(),
            "\"C:\\Example App\\Focus.exe\" --profile production --autostart --start-in-tray"
        );
        assert!(command(Path::new("Focus.exe"), Profile::Development, false).is_err());
        assert!(command(Path::new("C:\\bad\"name.exe"), Profile::Production, false).is_err());
        assert!(
            command(
                Path::new(&format!("C:\\{}\\focus.exe", "x".repeat(250))),
                Profile::Production,
                false
            )
            .is_err()
        );
        let extended = Path::new("\\\\?\\C:\\Example App\\Focus.exe");
        assert_eq!(
            command(path, Profile::Production, false).unwrap(),
            command(extended, Profile::Production, false).unwrap()
        );
        let unicode = Path::new("C:\\Example \u{03b1}\\Focus.exe");
        assert!(
            command(unicode, Profile::Development, false)
                .unwrap()
                .contains('\u{03b1}')
        );
        assert_ne!(name(Profile::Production), name(Profile::Development));
    }
    #[test]
    fn isolated_registry_roundtrip_preserves_other_values_and_profile_entries() {
        // Never exercise the user's real Run key in tests.
        let unique = tempfile::tempdir().unwrap();
        let id = unique.path().file_name().unwrap().to_str().unwrap();
        let path = format!("Software\\AttentionFocus\\Tests\\startup-{id}");
        let prod = name(Profile::Production);
        let dev = name(Profile::Development);
        assert_eq!(read_entry(&path, prod).unwrap(), None);
        set_entry(&path, "OtherExample", Some("Keep this synthetic value")).unwrap();
        set_entry(&path, prod, Some("Production example")).unwrap();
        set_entry(&path, dev, Some("Development example")).unwrap();
        assert_eq!(
            read_entry(&path, prod).unwrap().as_deref(),
            Some("Production example")
        );
        set_entry(&path, prod, None).unwrap();
        assert_eq!(read_entry(&path, prod).unwrap(), None);
        assert_eq!(
            read_entry(&path, dev).unwrap().as_deref(),
            Some("Development example")
        );
        assert_eq!(
            read_entry(&path, "OtherExample").unwrap().as_deref(),
            Some("Keep this synthetic value")
        );
        set_entry(&path, dev, None).unwrap();
        set_entry(&path, "OtherExample", None).unwrap();
        checked(unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, wide(&path).as_ptr()) }).unwrap();
    }
}
