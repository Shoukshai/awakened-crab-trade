use super::{game_input, native};
use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::*,
    System::{DataExchange::*, Memory::*},
    UI::WindowsAndMessaging::*,
};
static MUTATION: Mutex<()> = Mutex::new(());
static OWNER: OnceLock<Result<usize, String>> = OnceLock::new();
const UNICODE: u32 = 13;
fn owner() -> Result<HWND, String> {
    // Clipboard ownership receives synchronous Win32 messages. A capture worker
    // must not own an unpumped window or EmptyClipboard in the game can block.
    OWNER
        .get_or_init(|| {
            let (send, receive) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || unsafe {
                let window = CreateWindowExW(
                    0,
                    native::wide("STATIC").as_ptr(),
                    native::wide("APT clipboard").as_ptr(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                );
                if window.is_null() {
                    let _ = send.send(Err(native::error("Presse-papiers Windows")));
                    return;
                }
                if send.send(Ok(window as usize)).is_err() {
                    return;
                }
                let mut message: MSG = std::mem::zeroed();
                while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                DestroyWindow(window);
            });
            receive.recv().map_err(|e| e.to_string())?
        })
        .clone()
        .map(|window| window as HWND)
}
struct Open;
impl Open {
    fn new() -> Result<Self, String> {
        let start = Instant::now();
        let window = owner()?;
        loop {
            if unsafe { OpenClipboard(window) } != 0 {
                return Ok(Self);
            }
            if start.elapsed() > Duration::from_millis(300) {
                return Err(native::error("Presse-papiers occupé"));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for Open {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}
pub fn read() -> Result<String, String> {
    let _open = Open::new()?;
    unsafe {
        let h = GetClipboardData(UNICODE);
        if h.is_null() {
            return Err("Texte du presse-papiers indisponible.".into());
        }
        let bytes = GlobalSize(h);
        if bytes == 0 || bytes > 262144 {
            return Err("Texte du presse-papiers trop volumineux.".into());
        }
        let p = GlobalLock(h) as *const u16;
        if p.is_null() {
            return Err(native::error("Lecture du presse-papiers"));
        }
        let text = std::slice::from_raw_parts(p, bytes / 2);
        let end = text.iter().position(|c| *c == 0).unwrap_or(text.len());
        let result =
            String::from_utf16(&text[..end]).map_err(|_| "Texte UTF-16 invalide.".to_string());
        GlobalUnlock(h);
        result.and_then(|s| {
            if s.len() > 65536 {
                Err("Le presse-papiers dépasse 64 Kio.".into())
            } else {
                Ok(s)
            }
        })
    }
}
fn write(text: &str) -> Result<(), String> {
    let _open = Open::new()?;
    let text = native::wide(text);
    unsafe {
        let h = GlobalAlloc(GMEM_MOVEABLE, text.len() * 2);
        if h.is_null() {
            return Err(native::error("Allocation du presse-papiers"));
        }
        let p = GlobalLock(h) as *mut u16;
        if p.is_null() {
            GlobalFree(h);
            return Err(native::error("Écriture du presse-papiers"));
        }
        std::ptr::copy_nonoverlapping(text.as_ptr(), p, text.len());
        GlobalUnlock(h);
        if EmptyClipboard() == 0 || SetClipboardData(UNICODE, h).is_null() {
            GlobalFree(h);
            return Err(native::error("Écriture du presse-papiers"));
        }
        Ok(())
    }
}
fn restore(owned: &str, previous: Option<&str>) {
    if let Some(previous) = previous {
        if read().is_ok_and(|s| s == owned) {
            if let Err(e) = write(previous) {
                eprintln!("clipboard restoration failed: {e}");
            }
        }
    }
}
pub fn copy_from_game(
    context: &game_input::Context<'_>,
    restore_text: bool,
) -> Result<String, String> {
    context.check()?;
    let _lock = MUTATION.try_lock().map_err(|_| "Copie déjà en cours.")?;
    let guard = game_input::Guard::new(context)?;
    let previous = read().ok();
    guard.check()?;
    let marker = format!("__APT_RS_COPY_{}", std::process::id());
    write(&marker)?;

    let result = (|| {
        guard.copy()?;
        let start = Instant::now();
        let mut last_read = String::new();
        while start.elapsed() < Duration::from_millis(1500) {
            guard.check()?;
            {
                match read() {
                    Ok(text) => {
                        guard.check()?;
                        if text.starts_with("Item Class: ") {
                            if restore_text {
                                restore(&text, previous.as_deref());
                            }
                            return Ok(text);
                        }
                        last_read = format!("text available, {} UTF-8 bytes", text.len());
                    }
                    Err(error) => last_read = error,
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        eprintln!("Clipboard copy timed out: {last_read}");
        Err("Aucun nouvel objet copié. Survolez un objet dans PoE puis réessayez.".into())
    })();
    if result.is_err() {
        restore(&marker, previous.as_deref());
    }
    result
}
