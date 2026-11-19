#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
mod appearance;
mod calculated;
mod capture;
mod catalog;
mod compat;
#[cfg(target_os = "linux")]
mod desktop;
#[cfg(target_os = "windows")]
#[path = "desktop/windows/mod.rs"]
mod desktop;
#[cfg(any(target_os = "windows", feature = "neural-ocr", test))]
mod diagnostics;
mod economy;
mod edition;
mod filters;
mod hotkeys;
mod i18n;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod ipc;
#[cfg(any(test, feature = "cloud-ocr"))]
mod ocr;
mod paths;
#[cfg(target_os = "windows")]
mod windows_runtime;
// Retained for local diagnostics and compatibility with earlier map profiles.
#[allow(dead_code)]
mod inspection;
mod item;
mod item_filters;
mod leagues;
mod modifiers;
#[cfg(feature = "neural-ocr")]
mod neural_ocr;
mod overlay;
mod presets;
mod price_layout;
mod properties;
mod pseudo;
mod settings;
mod trade;
mod trade_item;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod ui;

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn main() {
    #[cfg(target_os = "windows")]
    if let Err(error) = windows_runtime::initialize() {
        windows_runtime::report_error(&error);
        std::process::exit(1);
    }
    if let Err(e) = ui::run() {
        #[cfg(target_os = "windows")]
        windows_runtime::report_error(&e);
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn main() {
    eprintln!(
        "Desktop backend not implemented for {}. Linux Wayland is the first supported backend.",
        std::env::consts::OS
    );
    std::process::exit(2);
}
