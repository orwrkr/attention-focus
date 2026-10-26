#[path = "src/icons.rs"]
mod icons;

fn main() {
    println!("cargo:rerun-if-changed=src/icons.rs");
    println!("cargo:rerun-if-changed=build.rs");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    for (name, state) in [
        ("app", icons::State::App),
        ("running", icons::State::Running),
        ("stopped", icons::State::Stopped),
    ] {
        std::fs::write(out.join(format!("{name}.ico")), icons::ico(state)).unwrap();
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon(out.join("app.ico").to_str().unwrap())
            .set("ProductName", "Attention Focus")
            .set("FileDescription", "Attention Focus")
            .set("InternalName", "attention-focus")
            .set("OriginalFilename", "attention-focus.exe");
        resource
            .compile()
            .expect("Windows icon and version resources must compile");
    }
}
