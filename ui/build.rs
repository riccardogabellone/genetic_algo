//! Embeds the icon and version details in the Windows executable, so Explorer and the
//! taskbar show the app's icon even before a window opens. Other platforms take their
//! icon from the package (`.desktop` file, macOS bundle), so this does nothing there.
//!
//! The `cfg(windows)` gate is on the build machine: a Windows binary cross-compiled from
//! Linux or macOS has no embedded icon. Release builds run on Windows.

fn main() {
    println!("cargo:rerun-if-changed=../assets/icons/icon.ico");
    // Both gates matter: `cfg(windows)` is the build machine (where `winresource` is
    // available), the variable is the target the binary is for.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_resources();
    }
}

#[cfg(windows)]
fn embed_windows_resources() {
    let mut resources = winresource::WindowsResource::new();
    resources
        .set_icon("../assets/icons/icon.ico")
        .set("ProductName", "Genetic Algo")
        .set(
            "FileDescription",
            "Genetic Algo: an agent-based evolution sandbox",
        );
    // A machine without a resource compiler (the Windows SDK's rc.exe, or MinGW's windres)
    // still builds the app, just without the embedded icon. GitHub's Windows runners have
    // the SDK, so release builds always get it.
    if let Err(error) = resources.compile() {
        println!("cargo:warning=the Windows icon was not embedded: {error}");
    }
}
