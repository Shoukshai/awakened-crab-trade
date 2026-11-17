//! Guarded XWayland text-box input; no window activation and no chat commands.
use std::{
    process::Command,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
use x11rb::{
    protocol::xproto::{AtomEnum, ConnectionExt},
    rust_connection::RustConnection,
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
    connection: RustConnection,
    window: u32,
}
impl<'a> Guard<'a> {
    pub fn new(context: &'a Context<'a>) -> Result<Self, String> {
        context.check()?;
        let (connection, _) = x11rb::connect(None).map_err(|e| e.to_string())?;
        let window = connection
            .get_input_focus()
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .focus;
        let r = Self {
            context,
            connection,
            window,
        };
        r.check()?;
        Ok(r)
    }
    pub fn check(&self) -> Result<(), String> {
        self.context.check()?;
        let c = &self.connection;
        let focus = c
            .get_input_focus()
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .focus;
        if focus != self.window {
            return Err("Focus XWayland modifié ; aucune touche supplémentaire envoyée.".into());
        }
        let class = c
            .get_property(false, focus, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .value;
        let name = c
            .get_property(false, focus, AtomEnum::WM_NAME, AtomEnum::ANY, 0, 256)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .value;
        if name != b"Path of Exile" || !class.split(|c| *c == 0).any(|s| s == b"steam_app_238960") {
            return Err("La fenêtre active n’est pas le PoE XWayland suivi.".into());
        }
        self.context.check()
    }
    pub fn key(&self, chord: &str) -> Result<(), String> {
        self.check()?;
        if !["ctrl+f", "ctrl+v", "Return", "ctrl+alt+c"].contains(&chord) {
            return Err("Action de saisie non prise en charge.".into());
        }
        let status = Command::new("xdotool")
            .args(["key", "--clearmodifiers", chord])
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("Échec de la saisie XWayland.".into())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_return_to_the_game_does_not_revive_an_older_input_request() {
        let active = AtomicBool::new(true);
        let generation = AtomicU64::new(4);
        let context = Context {
            active: &active,
            generation: &generation,
            token: 4,
        };
        assert!(context.check().is_ok());
        active.store(false, Ordering::Release);
        assert!(context.check().is_err());
        generation.store(5, Ordering::Release);
        active.store(true, Ordering::Release);
        assert!(context.check().is_err());
    }
}
