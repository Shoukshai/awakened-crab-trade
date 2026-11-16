//! First parser port: English item text and the upstream item catalogue.
//! Unsupported modifiers remain in the raw text and never become guessed filters.
#[path = "item/mercenary.rs"]
pub mod mercenary;
#[path = "item/special.rs"]
mod special;
#[path = "item/unidentified.rs"]
pub mod unidentified;
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
    #[serde(default)]
    pub stack_max: Option<u32>,
    pub influences: Vec<String>,
    pub properties: crate::properties::Properties,
    pub armour_base: crate::properties::ArmourBase,
    pub reference: String,
    pub unknown_modifiers: Vec<crate::modifiers::UnknownModifier>,
    pub map_area: Option<String>,
    pub unique_dust_value: Option<f64>,
    pub area: AreaProperties,
    #[serde(default)]
    pub logbook_areas: Vec<Vec<crate::modifiers::Modifier>>,
    #[serde(default)]
    pub mercenary: Option<mercenary::Parsed>,
    pub market: MarketMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketMetadata {
    pub known: bool,
    pub name: String,
    pub craftable: bool,
    pub exchangeable: bool,
    pub unique_base: Option<String>,
    pub gem: Option<GemMetadata>,
    pub vaal_gem: Option<GemIdentity>,
    pub synthesised: bool,
    pub fractured: bool,
    pub split: bool,
    pub foil: bool,
    pub foulborn: bool,
    pub vestigial: bool,
    pub veiled: bool,
    pub veiled_refs: Vec<String>,
    pub imbued_gem: bool,
    pub sentinel_charge: Option<u32>,
    pub linked_sockets: Option<u32>,
    pub fixed_stats: Option<Vec<String>>,
    pub random_mods: Vec<crate::modifiers::SourceInfo>,
    pub talisman_tier: Option<u32>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GemMetadata {
    pub max_level: u32,
    #[serde(default)]
    pub transfigured: bool,
    pub normal_variant: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemIdentity {
    pub name: String,
    pub reference: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AreaProperties {
    pub quantity: Option<f64>,
    pub rarity: Option<f64>,
    pub pack_size: Option<f64>,
    pub more_maps: Option<f64>,
    pub more_scarabs: Option<f64>,
    pub more_currency: Option<f64>,
    pub more_div_cards: Option<f64>,
    pub sulphur: Option<f64>,
    pub level: Option<u32>,
    pub completion_reward: Option<String>,
    pub job: Option<String>,
    pub job_level: Option<u32>,
    pub priceless: bool,
    pub wings_revealed: Option<u32>,
    pub wings_total: Option<u32>,
    pub explicit_mods: usize,
    pub special: bool,
    pub trade_disc: Option<String>,
}
impl AreaProperties {
    fn parse(lines: &[&str]) -> Self {
        let value = |key: &str| lines.iter().find_map(|s| s.strip_prefix(key));
        let number = |key: &str| {
            value(key).and_then(|s| {
                s.trim_start_matches('+')
                    .split(|c: char| !c.is_ascii_digit() && c != '-' && c != '.')
                    .next()?
                    .parse()
                    .ok()
            })
        };
        let job = lines
            .iter()
            .find_map(|s| s.strip_prefix("Requires ")?.split_once(" (Level "));
        let wings = value("Wings Revealed: ").and_then(|s| s.split_once('/'));
        Self {
            quantity: number("Item Quantity: "),
            rarity: number("Item Rarity: "),
            pack_size: number("Monster Pack Size: "),
            more_maps: number("More Maps: "),
            more_scarabs: number("More Scarabs: "),
            more_currency: number("More Currency: "),
            more_div_cards: number("More Divination Cards: "),
            sulphur: number("Dead Man's Sulphur: "),
            level: value("Area Level: ").and_then(super::item::number),
            completion_reward: value("Reward: Foil ").map(str::to_string),
            job: job.map(|(name, _)| name.to_string()),
            job_level: job.and_then(|(_, level)| super::item::number(level)),
            priceless: value("Heist Target: ").is_some_and(|s| s.ends_with("(Priceless)")),
            wings_revealed: wings.and_then(|(v, _)| super::item::number(v)),
            wings_total: wings.and_then(|(_, v)| super::item::number(v)),
            explicit_mods: lines
                .iter()
                .filter(|s| {
                    s.starts_with('{')
                        && (s.contains("Prefix Modifier") || s.contains("Suffix Modifier"))
                })
                .count(),
            special: false,
            trade_disc: None,
        }
    }
}
// Keep the complete upstream asset, but decode only fields used by the business
// layer. In particular icon URLs and unrelated area/build records need no JSON
// object trees in memory on every launch.
#[derive(Deserialize)]
struct CatalogueEntry {
    name: String,
    #[serde(rename = "refName")]
    reference: String,
    area: Option<AreaInfo>,
    namespace: String,
    #[serde(rename = "tradeTag")]
    trade_tag: Option<String>,
    #[serde(rename = "tradeDisc")]
    trade_discriminator: Option<String>,
    craftable: Option<Craftable>,
    #[serde(default)]
    exchangeable: bool,
    gem: Option<GemMetadata>,
    #[serde(rename = "mercenaryBuild")]
    mercenary_build: Option<mercenary::BuildRecord>,
    unique: Option<Unique>,
    #[serde(default)]
    armour: crate::properties::ArmourBase,
}
#[derive(Deserialize)]
struct AreaInfo {
    special: Option<bool>,
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
    #[serde(rename = "fixedStats")]
    fixed_stats: Option<Vec<String>>,
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

/// Name resolution used by RelatedItems.vue, including its unique-base fallback.
pub fn catalogue_name(namespace: &str, reference: &str, variant: Option<&str>) -> Option<String> {
    let entries = catalogue();
    if namespace == "UNIQUE" {
        if let Some(entry) = entries.iter().find(|e| {
            e.namespace == namespace
                && e.reference == reference
                && e.unique.as_ref().and_then(|u| u.base.as_deref()) == variant
        }) {
            return Some(entry.name.clone());
        }
    }
    entries
        .iter()
        .find(|e| e.namespace == namespace && e.reference == reference)
        .map(|e| e.name.clone())
}
fn number(text: &str) -> Option<u32> {
    text.replace(',', "")
        .split(|c: char| !c.is_ascii_digit())
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
        let mut name = names.first().ok_or("Missing item name")?.to_string();
        let unidentified = lines.contains(&"Unidentified");
        let foulborn = rarity == Rarity::Unique && !unidentified && name.starts_with("Foulborn ");
        if foulborn {
            name = name.strip_prefix("Foulborn ").unwrap().into();
        }
        let mut base = if matches!(rarity, Rarity::Rare | Rarity::Unique) && names.len() >= 2 {
            names[1].to_string()
        } else {
            name.clone()
        };
        let vestigial =
            rarity == Rarity::Unique && names.len() >= 2 && base.starts_with("Vestigial ");
        if vestigial {
            base = base.strip_prefix("Vestigial ").unwrap().into();
        }
        let synthesised = lines.contains(&"Synthesised Item");
        if synthesised {
            base = base.strip_prefix("Synthesised ").unwrap_or(&base).into();
        }
        let captured = lines.contains(&"Right-click to add this to your bestiary.");
        let metamorph =
            lines.contains(&"Combine this with four other different samples in Tane's Laboratory.");
        if metamorph {
            for suffix in ["Brain", "Eye", "Lung", "Heart", "Liver"] {
                if name.ends_with(&format!(" {suffix}")) {
                    base = format!("Metamorph {suffix}");
                    break;
                }
            }
        }
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
            _ if captured => "CAPTURED_BEAST",
            Rarity::Unique if !unidentified => "UNIQUE",
            Rarity::Gem => "GEM",
            Rarity::Divination => "DIVINATION_CARD",
            _ => "ITEM",
        };
        let identity = if namespace == "UNIQUE" { &name } else { &base };
        let info = catalogue().iter().find(|v| {
            v.namespace == namespace
                && v.name == *identity
                && (namespace != "UNIQUE"
                    || v.unique.as_ref().and_then(|u| u.base.as_deref()) == Some(base.as_str()))
        });
        let category_override = if captured {
            Some("Captured Beast")
        } else if metamorph {
            Some("Metamorph Sample")
        } else {
            match rarity {
                Rarity::Gem => Some("Gem"),
                Rarity::Currency => Some("Currency"),
                Rarity::Divination => Some("Divination Card"),
                _ => None,
            }
        };
        let category = category_override.or_else(|| {
            info.and_then(|v| v.craftable.as_ref().and_then(|c| c.category.as_deref()))
                .or_else(|| {
                    catalogue()
                        .iter()
                        .find(|v| v.namespace == "ITEM" && v.name == base)
                        .and_then(|v| v.craftable.as_ref().and_then(|c| c.category.as_deref()))
                })
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
        let reference = info
            .map(|v| v.reference.clone())
            .unwrap_or_else(|| identity.clone());
        let parsed = special::parse(text, &reference, category);
        let mut modifiers = parsed.modifiers;
        let mut unknown_modifiers = parsed.unknown;
        let mercenary = if reference == "Mercenary Warrant" {
            mercenary::parse(text)?
        } else {
            None
        };
        if let Some(m) = &mercenary {
            unknown_modifiers.extend(m.unknown.clone());
        }
        if [Some("Amulet"), Some("Belt"), Some("Ring"), Some("Quiver")].contains(&category) {
            let quality: Vec<_> = lines
                .iter()
                .filter_map(|line| line.strip_suffix(" (augmented)"))
                .filter_map(|line| {
                    crate::modifiers::translation::translate(line, "pseudo", None, true)
                })
                .filter(|m| crate::modifiers::translation::metadata(m).jewellery_quality)
                .collect();
            modifiers.splice(0..0, quality);
        }
        let map_area = lines
            .iter()
            .find_map(|l| l.strip_prefix("Map Area: "))
            .map(str::to_string)
            .or_else(|| {
                (category == Some("Chart"))
                    .then(|| {
                        lines
                            .iter()
                            .find(|line| {
                                catalogue()
                                    .iter()
                                    .any(|v| v.namespace == "AREA" && v.name == **line)
                            })
                            .map(|line| line.to_string())
                    })
                    .flatten()
            });
        let mut area = AreaProperties::parse(&lines);
        area.special = map_area
            .as_ref()
            .and_then(|name| {
                catalogue()
                    .iter()
                    .find(|v| v.namespace == "AREA" && v.name == *name)
            })
            .and_then(|v| v.area.as_ref())
            .and_then(|a| a.special)
            .unwrap_or(false);
        area.trade_disc = map_area
            .as_ref()
            .and_then(|name| {
                catalogue()
                    .iter()
                    .find(|v| v.namespace == "AREA" && v.name == *name)
            })
            .and_then(|v| v.trade_discriminator.clone());
        let mut veiled_refs = Vec::new();
        let mut header_name = None;
        let mut veiled = false;
        for line in &lines {
            if *line == "--------" {
                header_name = None;
            }
            if line.starts_with('{') {
                header_name = line.split('"').nth(1);
            }
            if ["Veiled Prefix", "Veiled Suffix"].contains(line) && header_name.is_some() {
                veiled = true;
                if let Some(m) = header_name.and_then(|name| {
                    crate::modifiers::translation::translate(name, "veiled", category, false)
                }) {
                    if !veiled_refs.contains(&m.reference) {
                        veiled_refs.push(m.reference);
                    }
                }
            }
        }
        let mut vaal_gem = None;
        let mut imbued_gem = false;
        if category == Some("Gem") {
            for section in normalized.split("--------").skip(1) {
                let section = section.trim();
                if section.lines().count() != 1 {
                    continue;
                }
                if let Some(gem) = catalogue()
                    .iter()
                    .find(|v| v.namespace == "GEM" && v.name == section)
                {
                    if vaal_gem.is_none() {
                        vaal_gem = Some(GemIdentity {
                            name: gem.name.clone(),
                            reference: gem.reference.clone(),
                        });
                    }
                }
                imbued_gem |=
                    crate::modifiers::translation::translate(section, "imbued", category, false)
                        .is_some();
            }
        }
        let market = MarketMetadata {
            known: info.is_some(),
            name: info
                .map(|v| v.name.clone())
                .unwrap_or_else(|| identity.clone()),
            craftable: info.is_some_and(|v| v.craftable.is_some()),
            exchangeable: info.is_some_and(|v| v.exchangeable),
            unique_base: info
                .and_then(|v| v.unique.as_ref())
                .and_then(|v| v.base.clone()),
            gem: info.and_then(|v| v.gem.clone()),
            vaal_gem,
            synthesised,
            fractured: modifiers.iter().any(|m| m.kind == "fractured")
                || lines.iter().any(|l| {
                    l.contains("Fractured Prefix Modifier")
                        || l.contains("Fractured Suffix Modifier")
                }),
            split: lines.contains(&"Split"),
            foil: rarity == Rarity::Unique && lines.contains(&"Foil Unique"),
            foulborn,
            vestigial,
            veiled,
            veiled_refs,
            imbued_gem,
            sentinel_charge: prop("Charge: "),
            linked_sockets: (links >= 5).then_some(links),
            fixed_stats: info
                .and_then(|v| v.unique.as_ref())
                .and_then(|u| u.fixed_stats.clone()),
            random_mods: lines
                .iter()
                .filter(|line| line.starts_with('{'))
                .map(|line| crate::modifiers::header_info(line))
                .filter(|m| {
                    ["explicit", "fractured", "veiled", "crafted"].contains(&m.kind.as_str())
                })
                .collect(),
            talisman_tier: prop("Talisman Tier: "),
        };
        let stack = lines
            .iter()
            .find_map(|s| s.strip_prefix("Stack Size: "))
            .map(|s| {
                s.chars()
                    .filter(|c| c.is_ascii_digit() || *c == '/')
                    .collect::<String>()
            });
        let (stack_size, stack_max) = stack
            .as_ref()
            .map(|s| {
                let mut parts = s.split('/');
                (
                    parts.next().and_then(|s| s.parse().ok()),
                    parts.next().and_then(|s| s.parse().ok()),
                )
            })
            .unwrap_or((None, None));
        Ok(Self {
            class,
            rarity,
            name,
            base,
            item_level: mercenary
                .as_ref()
                .and_then(|m| m.level)
                .or_else(|| prop("Item Level: ")),
            gem_level: prop("Level: "),
            quality: lines.iter().find_map(|line| {
                if [Some("Amulet"), Some("Belt"), Some("Ring"), Some("Quiver")].contains(&category)
                {
                    return line
                        .strip_suffix(" (augmented)")
                        .and_then(|line| {
                            crate::modifiers::translation::translate(line, "pseudo", None, true)
                        })
                        .filter(|m| crate::modifiers::translation::metadata(m).jewellery_quality)
                        .and_then(|m| m.value.map(|v| v as u32));
                }
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
            properties: {
                let mut p = crate::properties::Properties::parse(text);
                if !crate::properties::is_armour(category)
                    && !crate::properties::is_weapon(category)
                    && ![Some("Ring"), Some("Belt"), Some("Amulet"), Some("Quiver")]
                        .contains(&category)
                {
                    p.memory_strands = None;
                }
                p
            },
            armour_base,
            modifiers,
            unknown_modifiers,
            reference,
            map_area,
            unique_dust_value: info.and_then(|v| v.unique.as_ref().and_then(|u| u.dust)),
            area,
            logbook_areas: parsed.logbook_areas,
            mercenary,
            market,
            namespace: namespace.into(),
            trade_tag: info.and_then(|v| v.trade_tag.as_deref()).map(str::to_owned),
            trade_discriminator: info
                .and_then(|v| v.trade_discriminator.as_deref())
                .map(str::to_owned),
            category: category.map(str::to_owned),
            stack_size,
            stack_max,
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
        let markers = [
            (self.corrupted, "Corrupted"),
            (self.unidentified, "Unidentified"),
            (self.market.foulborn, "Foulborn"),
            (self.market.vestigial, "Vestigial"),
            (self.market.foil, "Foil Unique"),
        ]
        .into_iter()
        .filter(|(active, _)| *active)
        .map(|(_, text)| format!(" · {text}"))
        .collect::<String>();
        format!(
            "{}\n{} · {:?} · ilvl {}{}",
            self.name,
            self.base,
            self.rarity,
            self.item_level
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into()),
            markers
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
