//! First parser port: English item text and the upstream item catalogue.
//! Unknown modifiers remain visible; they are never silently converted to filters.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
    Gem,
    Currency,
    Divination,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub class: String,
    pub rarity: Rarity,
    pub name: String,
    pub base: String,
    pub item_level: Option<u32>,
    pub gem_level: Option<u32>,
    pub quality: Option<u32>,
    pub map_tier: Option<u32>,
    pub links: u32,
    pub corrupted: bool,
    pub unidentified: bool,
    pub raw: String,
    pub modifiers: Vec<crate::modifiers::Modifier>,
}
fn catalogue() -> &'static Vec<Value> {
    static ITEMS: OnceLock<Vec<Value>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        include_str!("../../renderer/public/data/en/items.ndjson")
            .lines()
            .map(|s| serde_json::from_str(s).expect("upstream catalogue entry"))
            .collect()
    })
}
fn number(text: &str) -> Option<u32> {
    text.split(|c: char| !c.is_ascii_digit())
        .find(|s| !s.is_empty())?
        .parse()
        .ok()
}
impl Item {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > 65536 {
            return Err("Item text exceeds 64 KiB".into());
        }
        let normalized = text.replace('\r', "");
        let lines: Vec<_> = normalized.lines().collect();
        let class = lines
            .first()
            .and_then(|s| s.strip_prefix("Item Class: "))
            .ok_or("Only English PoE item text is currently supported (expected Item Class)")?
            .to_string();
        let rarity = match lines.get(1).and_then(|s| s.strip_prefix("Rarity: ")) {
            Some("Normal") => Rarity::Normal,
            Some("Magic") => Rarity::Magic,
            Some("Rare") => Rarity::Rare,
            Some("Unique") => Rarity::Unique,
            Some("Gem") => Rarity::Gem,
            Some("Currency") => Rarity::Currency,
            Some("Divination Card") => Rarity::Divination,
            _ => return Err("Unsupported or missing item rarity".into()),
        };
        let names: Vec<_> = lines
            .iter()
            .skip(2)
            .take_while(|s| **s != "--------")
            .filter(|s| !s.is_empty())
            .copied()
            .collect();
        let name = names.first().ok_or("Missing item name")?.to_string();
        let mut base = if matches!(rarity, Rarity::Rare | Rarity::Unique) && names.len() >= 2 {
            names[1].to_string()
        } else {
            name.clone()
        };
        // Longest catalogue match avoids suffix/prefix ambiguity for magic items.
        if rarity == Rarity::Magic {
            base = catalogue()
                .iter()
                .filter(|v| v["namespace"] == "ITEM" && v.get("craftable").is_some())
                .filter_map(|v| v["name"].as_str())
                .filter(|s| {
                    name == *s
                        || name.starts_with(&format!("{s} "))
                        || name.ends_with(&format!(" {s}"))
                        || name.contains(&format!(" {s} "))
                })
                .max_by_key(|s| s.len())
                .ok_or("Unknown magic base type; refusing an inaccurate Trade query")?
                .to_string();
        }
        base = base.strip_prefix("Superior ").unwrap_or(&base).to_string();
        let prop = |key: &str| {
            lines
                .iter()
                .find_map(|s| s.strip_prefix(key).and_then(number))
        };
        let links = lines
            .iter()
            .find_map(|s| s.strip_prefix("Sockets: "))
            .map(|s| {
                s.split_whitespace()
                    .map(|g| g.split('-').count() as u32)
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        Ok(Self {
            class,
            rarity,
            name,
            base,
            item_level: prop("Item Level: "),
            gem_level: prop("Level: "),
            quality: prop("Quality: "),
            map_tier: prop("Map Tier: "),
            links,
            corrupted: lines.contains(&"Corrupted"),
            unidentified: lines.contains(&"Unidentified"),
            raw: text.to_string(),
            modifiers: crate::modifiers::parse(text),
        })
    }
    pub fn description(&self) -> String {
        format!(
            "{}\n{} · {:?} · ilvl {}{}",
            self.name,
            self.base,
            self.rarity,
            self.item_level
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            if self.corrupted { " · Corrupted" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_advanced_clipboard_item_preserves_identity_and_implicit() {
        let item = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        assert_eq!(item.rarity, Rarity::Normal);
        assert_eq!(item.base, "Driftwood Wand");
        assert_eq!(item.item_level, Some(1));
        assert_eq!(item.links, 2);
        assert_eq!(item.modifiers.len(), 1);
        assert_eq!(item.modifiers[0].id, "implicit.stat_2974417149");
        assert_eq!(item.modifiers[0].value, 8.);
    }
    #[test]
    fn rare_name_is_not_a_trade_base() {
        let i = Item::parse("Item Class: Rings\r\nRarity: Rare\r\nDoom Hold\r\nGold Ring\r\n--------\r\nItem Level: 83\r\nCorrupted\r\n").unwrap();
        assert_eq!(i.base, "Gold Ring");
        assert_eq!(i.item_level, Some(83));
        assert!(i.corrupted);
    }
    #[test]
    fn magic_base_comes_from_upstream() {
        let i = Item::parse("Item Class: Wands\nRarity: Magic\nLacquered Driftwood Wand of the Fox\n--------\nItem Level: 4").unwrap();
        assert_eq!(i.base, "Driftwood Wand");
    }
    #[test]
    fn sockets_gem_quality_and_level() {
        let i = Item::parse("Item Class: Skill Gems\nRarity: Gem\nFireball\n--------\nLevel: 20 (Max)\nQuality: +20% (augmented)\nSockets: R-G-B R-G B").unwrap();
        assert_eq!(i.links, 3);
        assert_eq!(i.gem_level, Some(20));
        assert_eq!(i.quality, Some(20));
    }
    #[test]
    fn rejects_clipboard_prose_unknown_language_and_incomplete_item() {
        for s in [
            "hello",
            "Classe d'objet: Anneaux\nRareté: Rare",
            "Item Class: Wands\nRarity: Magic",
        ] {
            assert!(Item::parse(s).is_err());
        }
    }
}
