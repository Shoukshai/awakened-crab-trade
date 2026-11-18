use super::native;
use serde::Serialize;
use std::{cell::RefCell, collections::HashMap};
use windows_sys::Win32::{
    Foundation::*,
    System::Threading::GetCurrentProcessId,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Window {
    pub id: u32,
    pub title: String,
    pub app_id: String,
    pub active: bool,
    pub minimized: bool,
    game: bool,
}
impl Window {
    pub fn is_game(&self) -> bool {
        self.game
    }
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct Snapshot {
    pub windows: Vec<Window>,
}
impl Snapshot {
    pub fn game_active(&self) -> bool {
        self.windows
            .iter()
            .any(|w| w.is_game() && w.active && !w.minimized)
    }
    pub fn other_active(&self) -> bool {
        self.windows.iter().any(|w| !w.is_game() && w.active)
    }
    pub fn game_present(&self) -> bool {
        self.windows.iter().any(|w| w.is_game() && !w.minimized)
    }
    pub fn game_exists(&self) -> bool {
        self.windows.iter().any(Window::is_game)
    }
}
type SnapshotCallback = Box<dyn Fn(Snapshot)>;
thread_local! {static SEND:RefCell<Option<SnapshotCallback>>=RefCell::new(None);static PREVIOUS:RefCell<Vec<Window>>=const{RefCell::new(Vec::new())};}
unsafe extern "system" fn enumerate(window: HWND, data: LPARAM) -> i32 {
    let out = &mut *(data as *mut Vec<Window>);
    let Some((pid, path)) = native::process(window) else {
        return 1;
    };
    if pid == GetCurrentProcessId() {
        return 1;
    }
    let game = native::is_target(window);
    let active = window == GetForegroundWindow();
    if !game && !active {
        return 1;
    }
    out.push(Window {
        id: native::id(window),
        title: native::title(window),
        app_id: path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        active,
        minimized: IsIconic(window) != 0 || IsWindowVisible(window) == 0,
        game,
    });
    1
}
fn publish() {
    let mut windows: Vec<Window> = Vec::new();
    unsafe {
        EnumWindows(Some(enumerate), &mut windows as *mut _ as LPARAM);
    }
    windows.sort_by_key(|w| w.id);
    PREVIOUS.with(|p| {
        if *p.borrow() != windows {
            p.replace(windows.clone());
            SEND.with(|s| {
                if let Some(send) = s.borrow().as_ref() {
                    send(Snapshot { windows });
                }
            });
        }
    });
}
unsafe extern "system" fn event(
    _: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    object: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if (event == EVENT_SYSTEM_FOREGROUND || object == OBJID_WINDOW)
        && (window.is_null()
            || event == EVENT_SYSTEM_FOREGROUND
            || native::is_target(window)
            || PREVIOUS.with(|p| p.borrow().iter().any(|w| w.id == native::id(window))))
    {
        publish();
    }
}
static THREAD: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
pub fn refresh() {
    let id = THREAD.load(std::sync::atomic::Ordering::Acquire);
    if id != 0 {
        unsafe {
            PostThreadMessageW(id, WM_APP + 81, 0, 0);
        }
    }
}
pub fn track(send: impl Fn(Snapshot) + 'static) -> Result<(), String> {
    SEND.with(|s| s.replace(Some(Box::new(send))));
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
        THREAD.store(
            windows_sys::Win32::System::Threading::GetCurrentThreadId(),
            std::sync::atomic::Ordering::Release,
        );
        let mut hooks = HashMap::new();
        for (first, last) in [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            (EVENT_OBJECT_CREATE, EVENT_OBJECT_HIDE),
        ] {
            let hook = SetWinEventHook(
                first,
                last,
                std::ptr::null_mut(),
                Some(event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if hook.is_null() {
                for hook in hooks.values() {
                    UnhookWinEvent(*hook);
                }
                return Err(native::error("Suivi des fenêtres Windows"));
            }
            hooks.insert(first, hook);
        }
        // Always publish the initial state, including absence of a game.
        PREVIOUS.with(|p| p.borrow_mut().clear());
        publish();
        SEND.with(|s| {
            if let Some(send) = s.borrow().as_ref() {
                send(Snapshot {
                    windows: PREVIOUS.with(|p| p.borrow().clone()),
                });
            }
        });
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            if msg.message == WM_APP + 81 {
                publish();
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        THREAD.store(0, std::sync::atomic::Ordering::Release);
        for hook in hooks.values() {
            UnhookWinEvent(*hook);
        }
    }
    Err("Suivi des fenêtres Windows interrompu.".into())
}
