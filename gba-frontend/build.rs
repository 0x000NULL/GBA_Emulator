//! Windows: embed `docs/branding/icons/strawberry.ico` and a version block into the exe so
//! Explorer and the taskbar show the mark. Everywhere else this does nothing. A missing
//! resource compiler (`rc.exe` / `windres`) only warns, so the build never depends on it.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../docs/branding/icons/strawberry.ico");
    #[cfg(windows)]
    embed_windows_resources();
}

#[cfg(windows)]
fn embed_windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../docs/branding/icons/strawberry.ico");
    res.set("ProductName", "Strawberry");
    res.set("FileDescription", "Strawberry, a Game Boy Advance emulator");
    if let Err(e) = res.compile() {
        println!("cargo:warning=exe icon not embedded (no rc.exe/windres?): {e}");
    }
}
