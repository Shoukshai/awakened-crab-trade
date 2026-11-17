//! Desktop integration is isolated from PoE and network policy.
//! Only wlroots layer-shell is implemented. Other backends must fail explicitly.
pub mod clipboard;
pub mod game_input;
pub mod wayland;
pub mod xwayland_shortcuts;

pub mod preferences;

pub mod game_geometry;

pub use wayland as tracker;
pub use xwayland_shortcuts as shortcuts;

pub fn open_url(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
