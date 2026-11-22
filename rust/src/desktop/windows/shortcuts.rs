//! A scoped low-level hook forwards every unrelated key unchanged.
use super::native;
use crate::hotkeys::{Action, Hotkey, Hotkeys};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
#[derive(Clone)]
struct Binding {
    key: u32,
    hotkey: Hotkey,
    action: Action,
}
struct Shared {
    bindings: RwLock<Vec<Binding>>,
    alive: AtomicBool,
    transaction: Mutex<()>,
}
pub struct Backend {
    shared: Arc<Shared>,
}
struct HookState {
    shared: Arc<Shared>,
    send: Box<dyn Fn(Action)>,
    suppressed: HashSet<u32>,
    last: HashMap<Action, Instant>,
}
thread_local! {static HOOK:RefCell<Option<HookState>>=const{RefCell::new(None)};}
fn key(h: &Hotkey) -> Result<u32, String> {
    let k = h.key.as_str();
    let value = match k {
        "Backspace" => VK_BACK,
        "Tab" => VK_TAB,
        "Enter" => VK_RETURN,
        "CapsLock" => VK_CAPITAL,
        "Escape" => VK_ESCAPE,
        "Space" => VK_SPACE,
        "PageUp" => VK_PRIOR,
        "PageDown" => VK_NEXT,
        "End" => VK_END,
        "Home" => VK_HOME,
        "ArrowLeft" => VK_LEFT,
        "ArrowUp" => VK_UP,
        "ArrowRight" => VK_RIGHT,
        "ArrowDown" => VK_DOWN,
        "Insert" => VK_INSERT,
        "Delete" => VK_DELETE,
        "PrintScreen" => VK_SNAPSHOT,
        "ScrollLock" => VK_SCROLL,
        "Pause" => VK_PAUSE,
        "NumLock" => VK_NUMLOCK,
        _ if k.len() == 1 && k.as_bytes()[0].is_ascii_alphanumeric() => {
            u16::from(k.as_bytes()[0].to_ascii_uppercase())
        }
        _ if k.starts_with('F') => {
            VK_F1
                + k[1..]
                    .parse::<u16>()
                    .map_err(|_| "Touche de fonction Windows invalide.")?
                - 1
        }
        _ if k.starts_with("Numpad") => {
            VK_NUMPAD0
                + k[6..]
                    .parse::<u16>()
                    .map_err(|_| "Touche numérique Windows invalide.")?
        }
        _ => return Err(format!("Raccourci Windows non pris en charge : {k}")),
    };
    Ok(u32::from(value))
}
fn bindings(keys: &Hotkeys) -> Result<Vec<Binding>, String> {
    keys.bindings()?
        .into_iter()
        .map(|(hotkey, action)| {
            Ok(Binding {
                key: key(&hotkey)?,
                hotkey,
                action,
            })
        })
        .collect()
}
fn matches(h: &Hotkey) -> bool {
    unsafe {
        h.ctrl == (GetAsyncKeyState(i32::from(VK_CONTROL)) < 0)
            && h.shift == (GetAsyncKeyState(i32::from(VK_SHIFT)) < 0)
            && h.alt == (GetAsyncKeyState(i32::from(VK_MENU)) < 0)
            && h.super_key
                == ([VK_LWIN, VK_RWIN]
                    .iter()
                    .any(|k| GetAsyncKeyState(i32::from(*k)) < 0))
    }
}
unsafe extern "system" fn callback(code: i32, kind: WPARAM, data: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(std::ptr::null_mut(), code, kind, data);
    }
    let event = &*(data as *const KBDLLHOOKSTRUCT);
    let down = kind as u32 == WM_KEYDOWN || kind as u32 == WM_SYSKEYDOWN;
    let up = kind as u32 == WM_KEYUP || kind as u32 == WM_SYSKEYUP;
    #[cfg(feature = "cloud-ocr")]
    if [u32::from(VK_MENU), u32::from(VK_LMENU), u32::from(VK_RMENU)].contains(&event.vkCode)
        && (down || up)
    {
        super::ocr_snapshot::alt_changed(event.vkCode, down);
    }
    if event.flags & LLKHF_INJECTED != 0 {
        return CallNextHookEx(std::ptr::null_mut(), code, kind, data);
    }
    let consumed = HOOK.with(|state| {
        let mut state = state.borrow_mut();
        let Some(state) = state.as_mut() else {
            return false;
        };
        if up {
            return state.suppressed.remove(&event.vkCode);
        }
        if !down {
            return false;
        }
        if state.suppressed.contains(&event.vkCode) {
            return true;
        }
        // While the passive panel leaves focus with PoE, Escape still dismisses it.
        // With an owned menu focused, GTK handles Escape to close only that menu.
        if event.vkCode == u32::from(VK_ESCAPE)
            && native::panel_visible()
            && native::focused_target().is_some()
        {
            state.suppressed.insert(event.vkCode);
            (state.send)(Action::Command("hide"));
            return true;
        }
        let binding = {
            let Ok(keys) = state.shared.bindings.read() else {
                return false;
            };
            keys.iter()
                .find(|b| b.key == event.vkCode && matches(&b.hotkey))
                .cloned()
        };
        let Some(binding) = binding else {
            return false;
        };
        let game = native::focused_target().is_some();
        let own = native::owned_active()
            && matches!(
                binding.action,
                Action::Command("hide" | "interact" | "settings")
            );
        if !game && !own {
            return false;
        }
        state.suppressed.insert(event.vkCode);
        let now = Instant::now();
        if binding.action.is_copy()
            && state
                .last
                .get(&binding.action)
                .is_some_and(|t| now.duration_since(*t) < Duration::from_millis(300))
        {
            return true;
        }
        state.last.insert(binding.action, now);
        (state.send)(binding.action);
        true
    });
    if consumed {
        1
    } else {
        CallNextHookEx(std::ptr::null_mut(), code, kind, data)
    }
}
impl Backend {
    pub fn new(keys: &Hotkeys) -> Result<Self, String> {
        Ok(Self {
            shared: Arc::new(Shared {
                bindings: RwLock::new(bindings(keys)?),
                alive: AtomicBool::new(true),
                transaction: Mutex::new(()),
            }),
        })
    }
    pub fn is_alive(&self) -> bool {
        self.shared.alive.load(Ordering::Acquire)
    }
    pub fn replace_and_notify<T>(
        &self,
        keys: &Hotkeys,
        commit: impl FnOnce() -> Result<T, String>,
        notify: impl FnOnce(&T, bool),
    ) -> Result<T, String> {
        let _guard = self.shared.transaction.lock().map_err(|e| e.to_string())?;
        let keys = bindings(keys)?;
        let value = commit()?;
        *self.shared.bindings.write().map_err(|e| e.to_string())? = keys;
        notify(&value, self.is_alive());
        Ok(value)
    }
    pub fn listen(
        &self,
        _active: Arc<AtomicBool>,
        send: impl Fn(Action) + Clone + Send + 'static,
    ) -> Result<(), String> {
        HOOK.with(|s| {
            s.replace(Some(HookState {
                shared: self.shared.clone(),
                send: Box::new(send),
                suppressed: HashSet::new(),
                last: HashMap::new(),
            }))
        });
        unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(callback),
                GetModuleHandleW(std::ptr::null()),
                0,
            );
            if hook.is_null() {
                self.shared.alive.store(false, Ordering::Release);
                return Err(native::error("Raccourcis Windows"));
            }
            eprintln!("shortcuts backend=Win32 scoped=true");
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            UnhookWindowsHookEx(hook);
        }
        self.shared.alive.store(false, Ordering::Release);
        Err("Raccourcis Windows interrompus.".into())
    }
}
