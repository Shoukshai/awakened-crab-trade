//! First parser port: English item text and the upstream item catalogue.
//! Unsupported modifiers remain in the raw text and never become guessed filters.
use serde::{Deserialize, Serialize};
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
    pub mirrored: bool,
    pub unmodifiable: bool,
    pub unidentified: bool,
    pub raw: String,
    pub modifiers: Vec<crate::modifiers::Modifier>,
    pub namespace: String,
    pub trade_tag: Option<String>,
    pub trade_discriminator: Option<String>,
    pub category: Option<String>,
    pub stack_size: Option<u32>,
    pub influences: Vec<String>,
    pub properties: crate::properties::Properties,
    pub armour_base: crate::properties::ArmourBase,
    pub reference: String,
    pub unknown_modifiers: Vec<crate::modifiers::UnknownModifier>,
    pub map_area: Option<String>,
    pub unique_dust_value: Option<f64>,
}
// Keep the complete upstream asset, but decode only fields used by the business
// layer. In particular icon URLs and unrelated area/build records need no JSON
// object trees in memory on every launch.
#[derive(Deserialize)]
struct CatalogueEntry {
    name: String,
    #[serde(rename = "refName")]
    reference: String,
    namespace: String,
    #[serde(rename = "tradeTag")]
    trade_tag: Option<String>,
    #[serde(rename = "tradeDisc")]
    trade_discriminator: Option<String>,
    craftable: Option<Craftable>,
    unique: Option<Unique>,
    #[serde(default)]
    armour: crate::properties::ArmourBase,
}
#[derive(Deserialize)]
struct Craftable {
    category: Option<String>,
}
#[derive(Deserialize)]
struct Unique {
    base: Option<String>,
    #[serde(rename = "disenchantValue")]
    dust: Option<f64>,
}
fn catalogue() -> &'static Vec<CatalogueEntry> {
    static ITEMS: OnceLock<Vec<CatalogueEntry>> = OnceLock::new();
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
                .filter(|v| v.namespace == "ITEM" && v.craftable.is_some())
                .map(|v| v.name.as_str())
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
        let mut named_map_tier = None;
        if class == "Maps" {
            named_map_tier = names
                .last()
                .and_then(|s| s.rsplit_once(" (Tier "))
                .and_then(|(_, tier)| tier.strip_suffix(')'))
                .and_then(|tier| tier.parse::<u32>().ok());
            if let Some((before, tier)) = base.rsplit_once(" (Tier ") {
                if let Some(tier) = tier.strip_suffix(')').and_then(|v| v.parse::<u32>().ok()) {
                    named_map_tier = Some(tier);
                    base = before.into();
                }
            }
        }
        let namespace = match rarity {
            Rarity::Unique => "UNIQUE",
            Rarity::Gem => "GEM",
            Rarity::Divination => "DIVINATION_CARD",
            _ => "ITEM",
        };
        let identity = if rarity == Rarity::Unique {
            &name
        } else {
            &base
        };
        let info = catalogue().iter().find(|v| {
            v.namespace == namespace
                && v.name == *identity
                && (rarity != Rarity::Unique
                    || v.unique.as_ref().and_then(|u| u.base.as_deref()) == Some(base.as_str()))
        });
        let category = info
            .and_then(|v| v.craftable.as_ref().and_then(|c| c.category.as_deref()))
            .or_else(|| {
                catalogue()
                    .iter()
                    .find(|v| v.namespace == "ITEM" && v.name == base)
                    .and_then(|v| v.craftable.as_ref().and_then(|c| c.category.as_deref()))
            });
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
        let armour_base = catalogue()
            .iter()
            .find(|v| v.namespace == "ITEM" && v.name == base)
            .map(|v| v.armour.clone())
            .unwrap_or_default();
        let (modifiers, unknown_modifiers) = crate::modifiers::parse_detailed(text, category);
        let reference = info
            .map(|v| v.reference.clone())
            .unwrap_or_else(|| identity.clone());
        let map_area = lines
            .iter()
            .find_map(|l| l.strip_prefix("Map Area: "))
            .map(str::to_string);
        Ok(Self {
            class,
            rarity,
            name,
            base,
            item_level: prop("Item Level: "),
            gem_level: prop("Level: "),
            quality: lines.iter().find_map(|line| {
                (line.starts_with("Quality: ") || line.starts_with("Quality ("))
                    .then(|| line.split_once(": ").and_then(|(_, value)| number(value)))
                    .flatten()
            }),
            map_tier: named_map_tier.or_else(|| prop("Map Tier: ")),
            links,
            corrupted: lines.contains(&"Corrupted") || lines.contains(&"Unmodifiable"),
            mirrored: lines.contains(&"Mirrored"),
            unmodifiable: lines.contains(&"Unmodifiable"),
            unidentified: lines.contains(&"Unidentified"),
            raw: text.to_string(),
            properties: crate::properties::Properties::parse(text),
            armour_base,
            modifiers,
            unknown_modifiers,
            reference,
            map_area,
            unique_dust_value: info.and_then(|v| v.unique.as_ref().and_then(|u| u.dust)),
            namespace: namespace.into(),
            trade_tag: info.and_then(|v| v.trade_tag.as_deref()).map(str::to_owned),
            trade_discriminator: info
                .and_then(|v| v.trade_discriminator.as_deref())
                .map(str::to_owned),
            category: category.map(str::to_owned),
            stack_size: prop("Stack Size: "),
            influences: [
                "Shaper", "Elder", "Crusader", "Hunter", "Redeemer", "Warlord",
            ]
            .into_iter()
            .filter(|name| lines.contains(&format!("{name} Item").as_str()))
            .map(str::to_owned)
            .collect(),
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
        assert_eq!(item.modifiers[0].value, Some(8.));
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
    fn unique_uses_base_category_for_local_stat_resolution() {
        let i=Item::parse("Item Class: Wands\nRarity: Unique\nLifesprig\nDriftwood Wand\n--------\nItem Level: 1\n--------\n{ Unique Modifier }\nAdds 7 to 13 Fire Damage").unwrap();
        assert_eq!(i.category.as_deref(), Some("Wand"));
        assert_eq!(i.modifiers[0].id, "explicit.stat_709508406");
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
