//! Portable shortcut names, compatible with upstream and resolved per backend.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Command(&'static str),
    StopwatchToggle(u64),
    StopwatchReset(u64),
    StashSearch(u64, u64),
}
impl Action {
    pub fn is_copy(self) -> bool {
        matches!(
            self,
            Self::Command("copy" | "inspect-copy" | "similar-copy" | "locked-copy")
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StopwatchKeys {
    pub id: u64,
    pub toggle: Option<String>,
    pub reset: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Hotkeys {
    #[serde(skip)]
    pub stash_scroll: bool,
    #[serde(skip)]
    pub stash_keys: Vec<(u64, u64, String)>,
    pub price: Option<String>,
    pub inspect: Option<String>,
    pub interact: Option<String>,
    pub hide: Option<String>,
    pub settings: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalogue: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delve: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similar: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stopwatch_keys: Vec<StopwatchKeys>,
}
impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            stash_scroll: false,
            stash_keys: Vec::new(),
            price: Some("Ctrl + D".into()),
            inspect: Some("Ctrl + Alt + D".into()),
            interact: Some("Shift + Space".into()),
            hide: Some("Ctrl + Shift + D".into()),
            settings: None,
            catalogue: None,
            delve: None,
            similar: None,
            stopwatch_keys: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
    pub key: String,
    pub keysym: u32,
}
pub const NAMED_KEYS: &[(&str, u32)] = &[
    ("Backspace", 0xff08),
    ("Tab", 0xff09),
    ("Enter", 0xff0d),
    ("CapsLock", 0xffe5),
    ("Escape", 0xff1b),
    ("Space", 0x20),
    ("PageUp", 0xff55),
    ("PageDown", 0xff56),
    ("End", 0xff57),
    ("Home", 0xff50),
    ("ArrowLeft", 0xff51),
    ("ArrowUp", 0xff52),
    ("ArrowRight", 0xff53),
    ("ArrowDown", 0xff54),
    ("Insert", 0xff63),
    ("Delete", 0xffff),
    ("NumpadMultiply", 0xffaa),
    ("NumpadAdd", 0xffab),
    ("NumpadSubtract", 0xffad),
    ("NumpadDecimal", 0xffae),
    ("NumpadDivide", 0xffaf),
    ("NumpadEnter", 0xff8d),
    ("Semicolon", b';' as u32),
    ("Equal", b'=' as u32),
    ("Comma", b',' as u32),
    ("Minus", b'-' as u32),
    ("Period", b'.' as u32),
    ("Slash", b'/' as u32),
    ("Backquote", b'`' as u32),
    ("BracketLeft", b'[' as u32),
    ("Backslash", b'\\' as u32),
    ("BracketRight", b']' as u32),
    ("Quote", b'\'' as u32),
];
impl Hotkey {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 128 {
            return Err("Raccourci trop long.".into());
        }
        let mut h = Self {
            ctrl: false,
            shift: false,
            alt: false,
            super_key: false,
            key: String::new(),
            keysym: 0,
        };
        for part in text.split('+').map(str::trim) {
            let modifier = match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => Some(&mut h.ctrl),
                "shift" => Some(&mut h.shift),
                "alt" => Some(&mut h.alt),
                "super" => Some(&mut h.super_key),
                _ => None,
            };
            if let Some(flag) = modifier {
                if *flag {
                    return Err("Modificateur répété dans le raccourci.".into());
                }
                *flag = true;
                continue;
            }
            if !h.key.is_empty() {
                return Err("Un raccourci doit contenir une seule touche principale.".into());
            }
            let (key, symbol) =
                key_name(part).ok_or_else(|| format!("Touche non prise en charge : {part:?}."))?;
            h.key = key;
            h.keysym = symbol;
        }
        if h.key.is_empty() {
            return Err("Ajoutez une touche principale au raccourci.".into());
        }
        Ok(h)
    }
    pub fn text(&self) -> String {
        [
            (self.ctrl, "Ctrl"),
            (self.shift, "Shift"),
            (self.alt, "Alt"),
            (self.super_key, "Super"),
        ]
        .into_iter()
        .filter(|(yes, _)| *yes)
        .map(|(_, s)| s)
        .chain(std::iter::once(self.key.as_str()))
        .collect::<Vec<_>>()
        .join(" + ")
    }
    pub fn from_keysym(
        keysym: u32,
        ctrl: bool,
        shift: bool,
        alt: bool,
        super_key: bool,
    ) -> Option<Self> {
        let keysym = match keysym {
            0xff9e => 0xffb0,
            0xff9c => 0xffb1,
            0xff99 => 0xffb2,
            0xff9b => 0xffb3,
            0xff96 => 0xffb4,
            0xff9d => 0xffb5,
            0xff98 => 0xffb6,
            0xff95 => 0xffb7,
            0xff97 => 0xffb8,
            0xff9a => 0xffb9,
            0xff9f => 0xffae,
            _ => keysym,
        };
        let key = if (b'a' as u32..=b'z' as u32).contains(&keysym)
            || (b'A' as u32..=b'Z' as u32).contains(&keysym)
            || (b'0' as u32..=b'9' as u32).contains(&keysym)
        {
            char::from_u32(keysym)?.to_ascii_uppercase().to_string()
        } else if (0xffbe..=0xffd5).contains(&keysym) {
            format!("F{}", keysym - 0xffbe + 1)
        } else if (0xffb0..=0xffb9).contains(&keysym) {
            format!("Numpad{}", keysym - 0xffb0)
        } else {
            NAMED_KEYS.iter().find(|(_, v)| *v == keysym)?.0.into()
        };
        let mut h = Self::parse(&key).ok()?;
        h.ctrl = ctrl;
        h.shift = shift;
        h.alt = alt;
        h.super_key = super_key;
        Some(h)
    }
}
fn key_name(part: &str) -> Option<(String, u32)> {
    if part.len() == 1 {
        let c = part.as_bytes()[0];
        if c.is_ascii_alphanumeric() {
            return Some((
                (c as char).to_ascii_uppercase().to_string(),
                c.to_ascii_lowercase() as u32,
            ));
        }
    }
    if let Some(n) = part
        .to_ascii_uppercase()
        .strip_prefix('F')
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| (1..=24).contains(n))
    {
        return Some((format!("F{n}"), 0xffbe + n - 1));
    }
    if let Some(n) = part
        .to_ascii_lowercase()
        .strip_prefix("numpad")
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n <= 9)
    {
        return Some((format!("Numpad{n}"), 0xffb0 + n));
    }
    NAMED_KEYS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(part))
        .map(|(name, sym)| ((*name).into(), *sym))
}
impl Hotkeys {
    pub fn disabled() -> Self {
        Self {
            stash_scroll: false,
            stash_keys: Vec::new(),
            price: None,
            inspect: None,
            interact: None,
            hide: None,
            settings: None,
            catalogue: None,
            delve: None,
            similar: None,
            stopwatch_keys: Vec::new(),
        }
    }
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
    pub fn bindings(&self) -> Result<Vec<(Hotkey, Action)>, String> {
        let mut seen = HashSet::new();
        let mut bindings = Vec::new();
        let mut raw = vec![
            (&self.price, Action::Command("copy")),
            (&self.inspect, Action::Command("locked-copy")),
            (&self.interact, Action::Command("interact")),
            (&self.hide, Action::Command("hide")),
            (&self.settings, Action::Command("settings")),
            (&self.catalogue, Action::Command("catalogue")),
            (&self.delve, Action::Command("delve")),
            (&self.similar, Action::Command("similar-copy")),
        ];
        let mut ids = HashSet::new();
        for keys in &self.stopwatch_keys {
            if keys.id == 0 || keys.id > 9_007_199_254_740_991 || !ids.insert(keys.id) {
                return Err("Identifiant de raccourci chronomètre invalide ou dupliqué.".into());
            }
            raw.push((&keys.toggle, Action::StopwatchToggle(keys.id)));
            raw.push((&keys.reset, Action::StopwatchReset(keys.id)));
        }
        let stash: Vec<_> = self
            .stash_keys
            .iter()
            .map(|(id, entry, text)| (Some(text.clone()), Action::StashSearch(*id, *entry)))
            .collect();
        for (key, action) in &stash {
            raw.push((key, *action));
        }
        for (value, command) in raw {
            if let Some(value) = value {
                let key = Hotkey::parse(value)?;
                let canonical = key.text();
                // Preserve upstream game reservations and the copy gesture used
                // by this backend. Binding it would intercept its own capture.
                if [
                    "Ctrl + C",
                    "Ctrl + Alt + C",
                    "Ctrl + V",
                    "Ctrl + A",
                    "Ctrl + F",
                    "Ctrl + Enter",
                    "Home",
                    "Delete",
                    "Enter",
                    "ArrowUp",
                    "ArrowRight",
                    "ArrowLeft",
                ]
                .contains(&canonical.as_str())
                {
                    return Err(format!(
                        "Le raccourci {canonical} est réservé au jeu ou à la copie."
                    ));
                }
                if !seen.insert(key.clone()) {
                    return Err(format!(
                        "Le raccourci {canonical} est attribué plusieurs fois."
                    ));
                }
                bindings.push((key, command));
            }
        }
        Ok(bindings)
    }
    pub fn normalized(&self) -> Result<Self, String> {
        self.bindings()?;
        let normalize = |s: &Option<String>| {
            s.as_ref()
                .map(|s| Hotkey::parse(s).map(|h| h.text()))
                .transpose()
        };
        Ok(Self {
            stash_scroll: self.stash_scroll,
            stash_keys: self.stash_keys.clone(),
            price: normalize(&self.price)?,
            inspect: normalize(&self.inspect)?,
            interact: normalize(&self.interact)?,
            hide: normalize(&self.hide)?,
            settings: normalize(&self.settings)?,
            catalogue: normalize(&self.catalogue)?,
            delve: normalize(&self.delve)?,
            similar: normalize(&self.similar)?,
            stopwatch_keys: self
                .stopwatch_keys
                .iter()
                .map(|k| {
                    Ok(StopwatchKeys {
                        id: k.id,
                        toggle: normalize(&k.toggle)?,
                        reset: normalize(&k.reset)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?
                .into_iter()
                .filter(|k| k.toggle.is_some() || k.reset.is_some())
                .collect(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn similar_search_is_optional_and_uses_the_capture_repeat_guard() {
        let mut h = Hotkeys::default();
        assert!(serde_json::to_value(&h).unwrap().get("similar").is_none());
        h.similar = Some("Ctrl + K".into());
        assert!(h
            .bindings()
            .unwrap()
            .iter()
            .any(|(_, a)| *a == Action::Command("similar-copy")));
        assert!(Action::Command("similar-copy").is_copy());
        h.similar = Some("Ctrl + D".into());
        assert!(h.bindings().is_err());
    }
    #[test]
    fn shortcut_names_roundtrip_with_every_modifier_combination() {
        let names: Vec<_> = NAMED_KEYS
            .iter()
            .map(|(s, _)| s.to_string())
            .chain(('A'..='Z').map(|c| c.to_string()))
            .chain((0..=9).map(|n| n.to_string()))
            .chain((1..=24).map(|n| format!("F{n}")))
            .chain((0..=9).map(|n| format!("Numpad{n}")))
            .collect();
        for name in names {
            for flags in 0..16 {
                let base = Hotkey::parse(&name).unwrap();
                let h = Hotkey::from_keysym(
                    base.keysym,
                    flags & 1 != 0,
                    flags & 2 != 0,
                    flags & 4 != 0,
                    flags & 8 != 0,
                )
                .unwrap();
                assert_eq!(Hotkey::parse(&h.text()).unwrap(), h);
            }
        }
    }
    #[test]
    fn duplicates_reserved_and_ambiguous_shortcuts_fail_before_registration() {
        for text in [
            "Ctrl",
            "Ctrl + Shift",
            "Ctrl + Ctrl + D",
            "Ctrl + D + G",
            "Ctrl + Unknown",
            "Ctrl + ",
            "F25",
            "Numpad10",
        ] {
            assert!(Hotkey::parse(text).is_err(), "{text}");
        }
        let mut h = Hotkeys {
            inspect: Some("Control+d".into()),
            ..Default::default()
        };
        assert!(h.bindings().is_err());
        for key in ["Ctrl + C", "Ctrl + Alt + C", "Ctrl + V", "Enter", "Home"] {
            h = Hotkeys::default();
            h.price = Some(key.into());
            assert!(h.bindings().is_err(), "{key}");
        }
        h = Hotkeys {
            stash_scroll: false,
            stash_keys: Vec::new(),
            price: None,
            inspect: None,
            interact: None,
            hide: None,
            settings: None,
            catalogue: None,
            delve: None,
            similar: None,
            stopwatch_keys: Vec::new(),
        };
        assert!(h.bindings().unwrap().is_empty());
        assert!(Hotkeys::default().normalized().unwrap().is_default());
    }
}
