use super::native;
use std::cell::RefCell;
use windows_sys::Win32::{
    Foundation::*,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};
pub fn focused() -> Option<(u32, crate::price_layout::Rect)> {
    let w = native::focused_game()?;
    Some((native::id(w), native::rect(w)?))
}
type GeometryCallback = Box<dyn FnMut(u32, crate::price_layout::Rect)>;
thread_local! {static SEND:RefCell<Option<GeometryCallback>>=RefCell::new(None);}
unsafe extern "system" fn event(
    _: HWINEVENTHOOK,
    _: u32,
    window: HWND,
    object: i32,
    child: i32,
    _: u32,
    _: u32,
) {
    if object == OBJID_WINDOW && child == 0 && native::is_game(window) {
        if let Some(rect) = native::rect(window) {
            SEND.with(|s| {
                if let Some(send) = s.borrow_mut().as_mut() {
                    send(native::id(window), rect);
                }
            });
        }
    }
}
pub fn watch(report: impl FnMut(u32, crate::price_layout::Rect) + 'static) -> Result<(), String> {
    SEND.with(|s| s.replace(Some(Box::new(report))));
    unsafe {
        let hook = SetWinEventHook(
            EVENT_OBJECT_LOCATIONCHANGE,
            EVENT_OBJECT_LOCATIONCHANGE,
            std::ptr::null_mut(),
            Some(event),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        if hook.is_null() {
            return Err(native::error("Suivi de la géométrie Windows"));
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        UnhookWinEvent(hook);
    }
    Err("Suivi de la géométrie interrompu.".into())
}
