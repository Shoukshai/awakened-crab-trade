//! Desktop integration is isolated from PoE and network policy.
//! Only wlroots layer-shell is implemented. Other backends must fail explicitly.
pub mod clipboard;
pub mod wayland;
pub mod xwayland_shortcuts;
