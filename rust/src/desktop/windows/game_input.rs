use super::native;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::GetForegroundWindow},
};
pub struct Context<'a> {
    pub active: &'a AtomicBool,
    pub generation: &'a AtomicU64,
    pub token: u64,
}
impl Context<'_> {
    pub fn check(&self) -> Result<(), String> {
        if self.active.load(Ordering::Acquire)
            && self.generation.load(Ordering::Acquire) == self.token
        {
            Ok(())
        } else {
            Err("Contexte de jeu perdu ; action annulée.".into())
        }
    }
}
pub struct Guard<'a> {
    context: &'a Context<'a>,
    window: HWND,
}
impl<'a> Guard<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        context.check()?;
        let window = native::focused_game().ok_or("La fenêtre active n’est pas PoE.")?;
        Ok(Self { context, window })
    }
    pub fn check(&self) -> Result<(), String> {
        self.context.check()?;
        unsafe {
            if GetForegroundWindow() != self.window || !native::is_game(self.window) {
                return Err("Fenêtre de jeu changée ; copie annulée.".into());
            }
        }
        Ok(())
    }
    pub fn copy(&self) -> Result<(), String> {
        let start = std::time::Instant::now();
        loop {
            self.check()?;
            let pressed = unsafe {
                [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
                    .iter()
                    .any(|k| GetAsyncKeyState(i32::from(*k)) < 0)
            };
            if !pressed {
                break;
            }
            if start.elapsed() > std::time::Duration::from_millis(1500) {
                return Err("Relâchez les touches du raccourci puis réessayez.".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
        self.check()?;
        let keys = [
            (VK_CONTROL, 0),
            (VK_MENU, 0),
            (b'C' as u16, 0),
            (b'C' as u16, KEYEVENTF_KEYUP),
            (VK_MENU, KEYEVENTF_KEYUP),
            (VK_CONTROL, KEYEVENTF_KEYUP),
        ];
        let input: Vec<INPUT> = keys
            .into_iter()
            .map(|(wVk, dwFlags)| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk,
                        wScan: 0,
                        dwFlags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            })
            .collect();
        let count = unsafe {
            SendInput(
                input.len() as u32,
                input.as_ptr(),
                std::mem::size_of::<INPUT>() as i32,
            )
        };
        if count != input.len() as u32 {
            return Err(native::error("Copie Ctrl+Alt+C Windows"));
        }
        self.check()
    }
}
