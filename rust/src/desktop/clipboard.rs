use std::{
    io::Write,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
pub fn read() -> Result<String, String> {
    let output = Command::new("timeout")
        .args(["2", "wl-paste", "--no-newline", "--type", "text"])
        .output()
        .map_err(|e| format!("wl-paste: {e}"))?;
    if !output.status.success() {
        return Err("Clipboard text unavailable (wl-paste/data-control)".into());
    }
    if output.stdout.len() > 65536 {
        return Err("Clipboard exceeds 64 KiB".into());
    }
    String::from_utf8(output.stdout).map_err(|_| "Clipboard is not UTF-8".into())
}
fn write(text: &str) -> Result<(), String> {
    let mut child = Command::new("wl-copy")
        .args(["--type", "text/plain;charset=utf-8"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("wl-copy stdin missing")?
        .write_all(text.as_bytes())
        .map_err(|e| e.to_string())?;
    if !child.wait().map_err(|e| e.to_string())?.success() {
        return Err("wl-copy failed".into());
    }
    Ok(())
}
/// Explicit user command only. Never runs while the overlay owns keyboard input.
/// The marker prevents accepting a previous item (also works around Proton 10+).
pub fn copy_from_game(active: &Arc<AtomicBool>) -> Result<String, String> {
    let previous = read().unwrap_or_default();
    if !active.load(Ordering::Acquire) {
        return Err("PoE is not active; copy cancelled".into());
    }
    let marker = format!("__APT_RS_COPY_{}", std::process::id());
    write(&marker)?;
    let focused = Command::new("xdotool")
        .args(["getwindowfocus", "getwindowname"])
        .output()
        .map_err(|e| e.to_string())?;
    if !active.load(Ordering::Acquire)
        || String::from_utf8_lossy(&focused.stdout).trim() != "Path of Exile"
    {
        if read().is_ok_and(|s| s == marker) {
            let _ = write(&previous);
        }
        return Err("PoE focus changed before copy; no keys sent".into());
    }
    // This is the XWayland game's clipboard gesture. It is not a general Wayland input backend.
    let status = Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+alt+c"])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("XWayland copy gesture failed".into());
    }
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1200) {
        if !active.load(Ordering::Acquire) {
            break;
        }
        if let Ok(text) = read() {
            if text.starts_with("Item Class: ") {
                return Ok(text);
            }
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    if read().is_ok_and(|s| s == marker) {
        let _ = write(&previous);
    }
    Err("No new item copied. Hover an item in PoE; alternatively Ctrl+Alt+C then run check.".into())
}
