mod calculated;
mod delve;
#[cfg(target_os = "linux")]
mod desktop;
mod economy;
mod filters;
mod hotkeys;
mod inspection;
mod item;
mod item_filters;
mod modifiers;
mod overlay;
mod presets;
mod properties;
mod pseudo;
mod settings;
mod stopwatch;
mod trade;
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
