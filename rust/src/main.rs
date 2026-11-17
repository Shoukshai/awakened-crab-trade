mod calculated;
#[cfg(target_os = "linux")]
mod desktop;
mod economy;
mod filters;
mod hotkeys;
// Retained for local diagnostics and compatibility with earlier map profiles.
#[allow(dead_code)]
mod inspection;
mod item;
mod item_filters;
mod leagues;
mod modifiers;
mod overlay;
mod presets;
mod price_layout;
mod properties;
mod pseudo;
mod settings;
mod trade;
mod trade_item;
#[cfg(target_os = "linux")]
mod ui;

#[cfg(target_os = "linux")]
fn main() {
    if let Err(e) = ui::run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!(
        "Desktop backend not implemented for {}. Linux Wayland is the first supported backend.",
        std::env::consts::OS
    );
    std::process::exit(2);
}
