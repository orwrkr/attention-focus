//! Fixed, independent runtime profiles. Never import old focus history implicitly.
use crate::source;
use std::{
    io,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Production,
    Development,
}
impl Profile {
    pub fn parse(arguments: &[String], debug: bool) -> io::Result<Self> {
        let profile = match arguments {
            [] => {
                if debug {
                    Self::Development
                } else {
                    Self::Production
                }
            }
            [flag, value] if flag == "--profile" => match value.as_str() {
                "production" => Self::Production,
                "development" => Self::Development,
                _ => {
                    return Err(io::Error::other(
                        "Profile must be production or development.",
                    ));
                }
            },
            _ => {
                return Err(io::Error::other(
                    "Usage: attention-focus [--profile production|development]",
                ));
            }
        };
        if debug && profile == Self::Production {
            return Err(io::Error::other(
                "Production requires a release build. Use the packaged EXE.",
            ));
        }
        Ok(profile)
    }
    pub fn folder(self) -> &'static str {
        match self {
            Self::Production => "Production",
            Self::Development => "Development",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Production => "Attention Focus",
            Self::Development => "Attention Focus (Development)",
        }
    }
    pub fn app_id(self) -> &'static str {
        match self {
            Self::Production => "AttentionFocus.Production",
            Self::Development => "AttentionFocus.Development",
        }
    }
}

pub struct Paths {
    pub root: PathBuf,
    pub directory: PathBuf,
    pub profile: Profile,
}
fn reject_link(path: &Path) -> io::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let linked = metadata.file_type().is_symlink();
    if linked {
        return Err(io::Error::other(
            "Linked application profile directories are not supported.",
        ));
    }
    Ok(())
}
impl Paths {
    /// Resolve identities before acquiring a mutex or opening writable files.
    pub fn new(root: &Path, profile: Profile) -> io::Result<Self> {
        reject_link(root)?;
        let root = source::resolve(root)?;
        let directory = root.join(profile.folder());
        reject_link(&directory)?;
        Ok(Self {
            root,
            directory,
            profile,
        })
    }
    pub fn validate_source(&self, path: &Path) -> io::Result<PathBuf> {
        source::validate_boundary(path, &self.root)
    }
    /// Import only the legacy source path, once; leave every legacy byte in place.
    pub fn prepare(&self) -> Result<(Option<source::Selection>, Option<String>)> {
        reject_link(&self.directory)?;
        if let Some(selection) = source::restore(&self.directory)? {
            self.validate_source(&selection.path)?;
            return Ok((Some(selection), None));
        }
        let legacy=match source::restore_source_only(&self.root) {
            Ok(selection)=>selection,
            Err(_)=>return Ok((None,Some("The previous MetaPlan setting could not be imported. Choose the Instance folder.".into()))),
        };
        if let Some(selection) = legacy {
            self.validate_source(&selection.path)?;
            match source::save(&self.directory, &selection.path) {
                Ok(selection) => return Ok((Some(selection), None)),
                Err(_) => return Ok((
                    None,
                    Some(
                        "The previous MetaPlan folder is unavailable. Choose the Instance folder."
                            .into(),
                    ),
                )),
            }
        }
        Ok((None, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_overrides_keep_debug_out_of_production() {
        assert_eq!(Profile::parse(&[], true).unwrap(), Profile::Development);
        assert_eq!(Profile::parse(&[], false).unwrap(), Profile::Production);
        assert!(Profile::parse(&["--profile".into(), "production".into()], true).is_err());
        assert_eq!(
            Profile::parse(&["--profile".into(), "development".into()], false).unwrap(),
            Profile::Development
        );
        assert!(Profile::parse(&["--data-dir".into(), "example".into()], false).is_err());
    }
    #[test]
    fn clean_profiles_preserve_legacy_bytes_and_remain_independent() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("focus.sqlite3"),
            b"Synthetic legacy history, never opened",
        )
        .unwrap();
        std::fs::write(root.join("window.ron"), b"Synthetic legacy geometry").unwrap();
        let prod = Paths::new(&root, Profile::Production).unwrap();
        let dev = Paths::new(&root, Profile::Development).unwrap();
        assert!(prod.prepare().unwrap().0.is_none());
        assert!(dev.prepare().unwrap().0.is_none());
        std::fs::create_dir_all(&prod.directory).unwrap();
        std::fs::create_dir_all(&dev.directory).unwrap();
        let prod_store =
            crate::tracking::Store::open(&prod.directory.join("focus.sqlite3")).unwrap();
        assert_eq!(prod_store.snapshot().unwrap().completed, 0);
        let mut engine = crate::tracking::Engine::new(
            crate::tracking::Store::open(&dev.directory.join("focus.sqlite3")).unwrap(),
            crate::tracking::Moment {
                wall_ms: 1000,
                mono_ms: 0,
            },
        );
        engine
            .apply(
                Some(crate::tracking::Command::Toggle),
                crate::tracking::Moment {
                    wall_ms: 1000,
                    mono_ms: 0,
                },
            )
            .unwrap();
        engine
            .apply(
                Some(crate::tracking::Command::Stop),
                crate::tracking::Moment {
                    wall_ms: 2000,
                    mono_ms: 1000,
                },
            )
            .unwrap();
        assert_eq!(
            engine
                .snapshot(crate::tracking::Moment {
                    wall_ms: 2000,
                    mono_ms: 1000
                })
                .unwrap()
                .total_ms,
            1000
        );
        assert_eq!(prod_store.snapshot().unwrap().total_ms, 0);
        drop(prod_store);
        let mut production = crate::tracking::Engine::new(
            crate::tracking::Store::open(&prod.directory.join("focus.sqlite3")).unwrap(),
            crate::tracking::Moment {
                wall_ms: 1000,
                mono_ms: 0,
            },
        );
        production
            .apply(
                Some(crate::tracking::Command::Toggle),
                crate::tracking::Moment {
                    wall_ms: 1000,
                    mono_ms: 0,
                },
            )
            .unwrap();
        production
            .apply(
                Some(crate::tracking::Command::Stop),
                crate::tracking::Moment {
                    wall_ms: 3000,
                    mono_ms: 2000,
                },
            )
            .unwrap();
        drop(production);
        prod.prepare().unwrap();
        assert_eq!(
            crate::tracking::Store::open(&prod.directory.join("focus.sqlite3"))
                .unwrap()
                .snapshot()
                .unwrap()
                .total_ms,
            2000
        );
        assert_eq!(
            engine
                .snapshot(crate::tracking::Moment {
                    wall_ms: 3000,
                    mono_ms: 2000
                })
                .unwrap()
                .total_ms,
            1000
        );
        assert_eq!(
            std::fs::read(root.join("focus.sqlite3")).unwrap(),
            b"Synthetic legacy history, never opened"
        );
        assert_eq!(
            std::fs::read(root.join("window.ron")).unwrap(),
            b"Synthetic legacy geometry"
        );
        assert!(prod.validate_source(&dev.directory).is_err());
        assert!(dev.validate_source(&prod.directory).is_err());
    }
    #[test]
    fn only_source_setting_is_imported_and_new_settings_are_independent() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("Instance");
        std::fs::create_dir_all(source.join("PROJECTS/Active")).unwrap();
        let root = temp.path().join("app");
        source::save(&root, &source).unwrap();
        let original = std::fs::read(root.join("source.sqlite3")).unwrap();
        let prod = Paths::new(&root, Profile::Production).unwrap();
        let dev = Paths::new(&root, Profile::Development).unwrap();
        assert_eq!(
            prod.prepare().unwrap().0.unwrap().path,
            source::resolve(&source).unwrap()
        );
        assert_eq!(
            dev.prepare().unwrap().0.unwrap().path,
            source::resolve(&source).unwrap()
        );
        let another = temp.path().join("OtherInstance");
        std::fs::create_dir_all(another.join("PROJECTS/Active")).unwrap();
        source::save(&dev.directory, &another).unwrap();
        assert_eq!(
            prod.prepare().unwrap().0.unwrap().path,
            source::resolve(&source).unwrap()
        );
        assert_eq!(
            dev.prepare().unwrap().0.unwrap().path,
            source::resolve(&another).unwrap()
        );
        assert_eq!(
            std::fs::read(root.join("source.sqlite3")).unwrap(),
            original
        );
        assert!(!prod.directory.join("focus.sqlite3").exists());
    }

    #[cfg(windows)]
    #[test]
    fn linked_development_cannot_open_production_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        let prod = Paths::new(&root, Profile::Production).unwrap();
        std::fs::create_dir_all(&prod.directory).unwrap();
        std::fs::write(prod.directory.join("sentinel"), b"Production content").unwrap();
        let link = prod.root.join("Development");
        let status=std::process::Command::new("powershell.exe")
            .args(["-NoProfile","-NonInteractive","-Command","$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:AF_TEST_LINK -Target $env:AF_TEST_TARGET | Out-Null"])
            .env("AF_TEST_LINK",&link).env("AF_TEST_TARGET",&prod.directory).status().unwrap();
        assert!(status.success());
        assert!(Paths::new(&root, Profile::Development).is_err());
        assert!(prod.prepare().is_ok());
        assert_eq!(
            std::fs::read(prod.directory.join("sentinel")).unwrap(),
            b"Production content"
        );
        std::fs::remove_dir(link).unwrap();
    }
}
