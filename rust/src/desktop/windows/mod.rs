//! Native Windows integration. UI, parsing and market services remain shared.
pub mod clipboard;
pub mod game_geometry;
pub mod game_input;
pub mod native;
#[cfg(feature = "cloud-ocr")]
pub mod ocr;
#[cfg(feature = "cloud-ocr")]
pub mod ocr_snapshot;
#[path = "../preferences.rs"]
pub mod preferences;
#[cfg(feature = "cloud-ocr")]
mod screen_capture;
pub mod shortcuts;
pub mod tracker;
#[cfg(feature = "cloud-ocr")]
mod window_capture;

pub fn open_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" {
        return Err("Lien externe non pris en charge.".into());
    }
    unsafe {
        let result = windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            native::wide("open").as_ptr(),
            native::wide(url).as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
        if result as isize <= 32 {
            Err(native::error("Ouverture du navigateur"))
        } else {
            Ok(())
        }
    }
}
