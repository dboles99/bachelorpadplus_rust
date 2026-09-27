//! Windows resources for the executable (release plan M2-02): the icon
//! Explorer and the taskbar show, VERSIONINFO for the file's Properties
//! page, and the application manifest (`app.manifest`, which says why each
//! setting is there).
//!
//! Only when building **on** Windows **for** Windows. The resource compiler
//! is a Windows SDK tool, and a cross-check from Linux
//! (`cargo clippy --target x86_64-pc-windows-msvc`) has none; skipping
//! there keeps that check possible and costs nothing, because releases are
//! built on Windows (ADR-0016). The Windows gate asserts the result.

fn main() -> std::io::Result<()> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=app.manifest");
    println!("cargo::rerun-if-changed=../../assets/icon/bachelorpad.ico");

    let for_windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    if !(for_windows && cfg!(windows)) {
        return Ok(());
    }

    let mut resource = winresource::WindowsResource::new();
    // FileVersion and ProductVersion come from CARGO_PKG_VERSION by
    // default, so the Properties page cannot disagree with `--version`.
    resource
        .set_icon("../../assets/icon/bachelorpad.ico")
        .set_manifest_file("app.manifest")
        .set("ProductName", "BachelorPad+")
        .set("FileDescription", "BachelorPad+")
        .set("InternalName", "bachelorpad")
        .set("OriginalFilename", "bachelorpad.exe")
        .set("CompanyName", "Daniel Boles")
        .set(
            "LegalCopyright",
            "Copyright (c) 2026 Daniel Boles; MIT OR Apache-2.0",
        );
    resource.compile()
}
