#[path = "src/icons.rs"]
mod icons;

fn main() {
    println!("cargo:rerun-if-changed=src/icons.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=third-party/THIRD-PARTY-NOTICES.txt");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    // The notices travel inside the EXE, and repeat the same license texts many times.
    let notices = std::fs::read("third-party/THIRD-PARTY-NOTICES.txt").unwrap();
    std::fs::write(
        out.join("notices.deflate"),
        miniz_oxide::deflate::compress_to_vec(&notices, 9),
    )
    .unwrap();
    for (name, state) in [
        ("app", icons::State::App),
        ("app-development", icons::State::AppDevelopment),
        ("running", icons::State::Running),
        ("stopped", icons::State::Stopped),
        ("running-development", icons::State::RunningDevelopment),
        ("stopped-development", icons::State::StoppedDevelopment),
    ] {
        std::fs::write(out.join(format!("{name}.ico")), icons::ico(state)).unwrap();
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    std::fs::write(out.join("icon-version.txt"), &version).unwrap();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let development = std::env::var("PROFILE").as_deref() == Ok("debug");
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon(
                out.join(if development {
                    "app-development.ico"
                } else {
                    "app.ico"
                })
                .to_str()
                .unwrap(),
            )
            .set("ProductName", "Attention Focus")
            .set(
                "FileDescription",
                if development {
                    "Attention Focus (Development)"
                } else {
                    "Attention Focus"
                },
            )
            .set("FileVersion", &format!("{version}.0"))
            .set(
                "ProductVersion",
                &if development {
                    format!("{version}-dev")
                } else {
                    version.clone()
                },
            )
            .set("InternalName", "attention-focus")
            .set("OriginalFilename", "attention-focus.exe");
        resource
            .compile()
            .expect("Windows icon and version resources must compile");
    }
}
