//! Windows: embed `docs/branding/icons/strawberry.ico` and a version block into the exe so
//! Explorer and the taskbar show the mark (`basket_build::windows_icon`). Everywhere else this
//! does nothing. A missing resource compiler (`rc.exe` / `windres`) only warns.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    basket_build::windows_icon("../docs/branding/icons/strawberry.ico", "Strawberry", "Strawberry, a Game Boy Advance emulator");
}
