//! Build identity is independent of filenames and editable preferences.
pub const fn is_ocr() -> bool {
    cfg!(all(target_os = "windows", feature = "cloud-ocr"))
}
pub const fn id() -> &'static str {
    if is_ocr() {
        "awakened-crab-trade-ocr"
    } else {
        "awakened-crab-trade"
    }
}
pub const fn title() -> &'static str {
    if is_ocr() {
        "Awakened Crab Trade OCR"
    } else {
        "Awakened Crab Trade"
    }
}
pub fn capture_mode() -> crate::capture::Mode {
    if is_ocr() {
        crate::capture::Mode::Ocr
    } else {
        crate::capture::Mode::Clipboard
    }
}

#[cfg(target_os = "windows")]
pub const fn mutex_name() -> &'static str {
    if is_ocr() {
        "Local\\AwakenedCrabTrade.OCR.Control"
    } else {
        "Local\\AwakenedCrabTrade.Control"
    }
}
