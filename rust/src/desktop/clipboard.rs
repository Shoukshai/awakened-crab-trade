use std::{
    io::Write,
    process::{Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};
static MUTATION: Mutex<()> = Mutex::new(());
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
pub fn copy_from_game(
    context: &super::game_input::Context<'_>,
    restore: bool,
) -> Result<String, String> {
    context.check()?;
    let _mutation = MUTATION
        .try_lock()
        .map_err(|_| "Clipboard action already in progress")?;
    let guard = super::game_input::Guard::new(context)?;
    let previous = read().ok();
    guard.check()?;
    let marker = format!("__APT_RS_COPY_{}", std::process::id());
    write(&marker)?;
    let result = (|| {
        guard.key("ctrl+alt+c")?;
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(1200) {
            context.check()?;
            if let Ok(text) = read() {
                context.check()?;
                if text.starts_with("Item Class: ") {
                    if restore {
                        restore_if_unchanged(&text, previous.as_deref());
                    }
                    return Ok(text);
                }
            }
            std::thread::sleep(Duration::from_millis(40));
        }
        Err(
            "No new item copied. Hover an item in PoE; alternatively Ctrl+Alt+C then run check."
                .into(),
        )
    })();
    if result.is_err() {
        restore_if_unchanged(&marker, previous.as_deref());
    }
    result
}
fn restore_if_unchanged(owned: &str, previous: Option<&str>) {
    if let Some(previous) = previous {
        if read().is_ok_and(|s| s == owned) {
            if let Err(e) = write(previous) {
                eprintln!("clipboard text restoration failed: {e}");
            }
        }
    }
}
/// Explicit search only. The caller first hides the overlay and observes game focus.
/// Hold the mutation gate for the upstream 120 ms throttling/restoration interval.
pub fn stash_search(
    text: &str,
    restore: bool,
    context: &super::game_input::Context<'_>,
) -> Result<(), String> {
    crate::stash_search::validate_text(text)?;
    let _mutation = MUTATION
        .try_lock()
        .map_err(|_| "Une action de presse-papiers est déjà en cours.")?;
    let guard = super::game_input::Guard::new(context)?;
    let previous = if restore { read().ok() } else { None };
    guard.check()?;
    write(text)?;
    let started = Instant::now();
    let result = (|| {
        guard.key("ctrl+f")?;
        guard.key("ctrl+v")?;
        guard.key("Return")
    })();
    if let Some(delay) = Duration::from_millis(120).checked_sub(started.elapsed()) {
        std::thread::sleep(delay);
    }
    // Never overwrite a clipboard selection changed by the user in the meantime.
    if restore {
        restore_if_unchanged(text, previous.as_deref());
    }
    result
}
