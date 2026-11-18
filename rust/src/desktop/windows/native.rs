use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
static PANEL_VISIBLE: AtomicBool = AtomicBool::new(false);
pub fn set_panel_visible(visible: bool) {
    PANEL_VISIBLE.store(visible, Ordering::Release);
}
pub fn panel_visible() -> bool {
    PANEL_VISIBLE.load(Ordering::Acquire)
}
use windows_sys::Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*};
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn error(action: &str) -> String {
    format!("{action}: {}", std::io::Error::last_os_error())
}
pub fn hwnd(id: u32) -> HWND {
    id as usize as HWND
}
pub fn id(window: HWND) -> u32 {
    window as usize as u32
}
pub fn process(window: HWND) -> Option<(u32, PathBuf)> {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(window, &mut pid);
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut text = vec![0u16; 32768];
        let mut n = text.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, text.as_mut_ptr(), &mut n);
        CloseHandle(handle);
        (ok != 0).then(|| {
            (
                pid,
                PathBuf::from(String::from_utf16_lossy(&text[..n as usize])),
            )
        })
    }
}
pub fn fixture_root() -> Option<PathBuf> {
    crate::paths::private_windows_test_root()
}
pub fn is_game(window: HWND) -> bool {
    unsafe {
        if window.is_null() || IsWindow(window) == 0 {
            return false;
        }
        let mut class = [0u16; 128];
        let n = GetClassNameW(window, class.as_mut_ptr(), class.len() as i32);
        if n <= 0 || String::from_utf16_lossy(&class[..n as usize]) != "POEWindowClass" {
            return false;
        }
    }
    let Some((_, path)) = process(window) else {
        return false;
    };
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(
        name.as_str(),
        "pathofexile.exe"
            | "pathofexile_x64.exe"
            | "pathofexilesteam.exe"
            | "pathofexile_x64steam.exe"
            | "pathofexile_kg.exe"
            | "pathofexile_x64_kg.exe"
    ) {
        return true;
    }
    name == "apt-win32-fixture.exe"
        && fixture_root()
            .is_some_and(|root| path.parent().and_then(|p| p.canonicalize().ok()) == Some(root))
}
pub fn title(window: HWND) -> String {
    unsafe {
        let mut s = vec![0u16; 512];
        let n = GetWindowTextW(window, s.as_mut_ptr(), s.len() as i32);
        String::from_utf16_lossy(&s[..n.max(0) as usize])
    }
}
pub fn owned_active() -> bool {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
        pid == GetCurrentProcessId()
    }
}
pub fn focused_game() -> Option<HWND> {
    unsafe {
        let w = GetForegroundWindow();
        is_game(w).then_some(w)
    }
}
pub fn rect(window: HWND) -> Option<crate::price_layout::Rect> {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        let mut origin = POINT { x: 0, y: 0 };
        if GetClientRect(window, &mut rect) == 0
            || windows_sys::Win32::Graphics::Gdi::ClientToScreen(window, &mut origin) == 0
        {
            return None;
        }
        (rect.right > rect.left && rect.bottom > rect.top).then_some(crate::price_layout::Rect {
            x: origin.x,
            y: origin.y,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        })
    }
}

static CAPTURE: std::sync::OnceLock<std::sync::RwLock<crate::capture::Settings>> =
    std::sync::OnceLock::new();
pub fn set_capture(settings: &crate::capture::Settings) {
    *CAPTURE.get_or_init(Default::default).write().unwrap() = settings.clone();
    super::tracker::refresh();
}
pub fn is_target(window: HWND) -> bool {
    let settings = CAPTURE.get_or_init(Default::default).read().unwrap();
    if settings.mode == crate::capture::Mode::Clipboard {
        return is_game(window);
    }
    let Some((pid, path)) = process(window) else {
        return false;
    };
    if pid == unsafe { GetCurrentProcessId() } {
        return false;
    }
    let desired = settings
        .executable
        .trim()
        .replace('/', "\\")
        .to_ascii_lowercase();
    if desired.contains('\\') {
        path.to_string_lossy().to_ascii_lowercase() == desired
    } else {
        path.file_name()
            .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(&desired))
    }
}
pub fn focused_target() -> Option<HWND> {
    let window = unsafe { GetForegroundWindow() };
    is_target(window).then_some(window)
}
