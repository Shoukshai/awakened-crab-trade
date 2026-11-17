//! Portable stash-search widget data from WidgetStashSearch.vue.
use crate::widget_menu::{Anchor, Config as Position};
use serde::{Deserialize, Serialize};
pub const MAX_WIDGETS: usize = 16;
pub const MAX_ENTRIES: usize = 64;
pub const MAX_ID: u64 = 9_007_199_254_740_991;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub id: u64,
    pub name: String,
    pub text: String,
    pub hotkey: Option<String>,
}
impl Entry {
    pub fn label(&self) -> &str {
        if self.name.is_empty() {
            &self.text
        } else {
            &self.name
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub id: u64,
    pub title: String,
    pub wanted: bool,
    pub x: f64,
    pub y: f64,
    pub anchor: Anchor,
    pub enable_hotkeys: bool,
    pub entries: Vec<Entry>,
}
impl Default for Config {
    fn default() -> Self {
        Self::new(1, 0., 0.)
    }
}
impl Config {
    pub fn new(id: u64, random_x: f64, random_y: f64) -> Self {
        Self {
            id,
            title: String::new(),
            wanted: true,
            x: 20. + random_x * 20.,
            y: 20. + random_y * 20.,
            anchor: Anchor::Tl,
            enable_hotkeys: true,
            entries: vec![Entry {
                id: 1,
                name: String::new(),
                text: "Currency".into(),
                hotkey: None,
            }],
        }
    }
    pub fn position(&self) -> Position {
        Position {
            x: self.x,
            y: self.y,
            anchor: self.anchor,
            always_show: false,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.id == 0 || self.id > MAX_ID || self.title.chars().count() > 100 {
            return Err("Identifiant ou titre de recherche coffre invalide.".into());
        }
        self.position().validate()?;
        if self.entries.len() > MAX_ENTRIES {
            return Err("Au plus 64 filtres par recherche coffre.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for e in &self.entries {
            if e.id == 0 || e.id > MAX_ID || !ids.insert(e.id) || e.name.chars().count() > 100 {
                return Err("Nom ou identifiant de filtre coffre invalide ou dupliqué.".into());
            }
            validate_text(&e.text)?;
            if let Some(h) = &e.hotkey {
                crate::hotkeys::Hotkey::parse(h)?;
            }
        }
        Ok(())
    }
    pub fn add_entry(&mut self) -> Result<u64, String> {
        let id = self.entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        if self.entries.len() >= MAX_ENTRIES || id > MAX_ID {
            return Err("Limite des filtres coffre atteinte.".into());
        }
        self.entries.push(Entry {
            id,
            name: String::new(),
            text: String::new(),
            hotkey: None,
        });
        Ok(id)
    }
}
pub fn validate_text(text: &str) -> Result<(), String> {
    if text.encode_utf16().count() > 250 || text.chars().any(char::is_control) {
        Err("Le filtre coffre doit tenir sur une ligne de 250 caractères UTF-16 maximum.".into())
    } else {
        Ok(())
    }
}
pub fn defaults() -> Vec<Config> {
    [
        (
            "Map rolling",
            35.,
            46.,
            vec![
                "\"Pack Size: +3\"",
                "Reflect",
                "\"Cannot Leech Life\"",
                "\"Cannot Leech Mana\"",
            ],
        ),
        (
            "Dump sorting",
            34.,
            56.,
            vec![
                "Currency",
                "\"Divination Card\"",
                "Fossil",
                "\"Map Tier\"",
                "\"Map Device\" \"Rarity: Normal\"",
                "Tane Laboratory",
            ],
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (title, x, y, texts))| Config {
        id: i as u64 + 1,
        title: title.into(),
        wanted: false,
        x,
        y,
        entries: texts
            .into_iter()
            .enumerate()
            .map(|(i, text)| Entry {
                id: i as u64 + 1,
                name: String::new(),
                text: text.into(),
                hotkey: None,
            })
            .collect(),
        ..Config::default()
    })
    .collect()
}
pub fn is_default(configs: &[Config]) -> bool {
    configs == defaults()
}
pub fn similar_filter(name: &str) -> String {
    serde_json::to_string(name).expect("serializing a string cannot fail")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_instances_keep_upstream_positions_entries_and_monotone_entry_ids() {
        let mut c = Config::new(7, 0.25, 0.75);
        assert_eq!((c.x, c.y), (25., 35.));
        assert!(c.wanted && c.enable_hotkeys);
        assert_eq!(c.entries[0].label(), "Currency");
        c.entries[0].id = 4;
        assert_eq!(c.add_entry().unwrap(), 5);
        c.entries[1].name = "Nom".into();
        assert_eq!(c.entries[1].label(), "Nom");
        c.validate().unwrap();
    }
    #[test]
    fn utf16_limits_and_duplicate_ids_fail_without_silent_truncation() {
        assert!(validate_text(&"😀".repeat(125)).is_ok());
        assert!(validate_text(&"😀".repeat(126)).is_err());
        assert!(validate_text("Currency\n/command").is_err());
        let mut c = Config::default();
        c.entries.push(c.entries[0].clone());
        assert!(c.validate().is_err());
        for c in defaults() {
            c.validate().unwrap();
        }
    }
}
