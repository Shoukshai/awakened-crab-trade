//! Heist-target search and selection from WidgetItemSearch.vue. No clipboard,
//! game input or network is performed by this module.
use super::{catalogue, AreaProperties, Item, MarketMetadata, Rarity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Gem,
    Replica,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Choice {
    pub id: usize,
    pub name: String,
    pub reference: String,
    pub namespace: String,
    pub base: Option<String>,
}
impl Choice {
    fn from_index(id: usize) -> Self {
        let entry = &catalogue()[id];
        Self {
            id,
            name: entry.name.clone(),
            reference: entry.reference.clone(),
            namespace: entry.namespace.clone(),
            base: entry.unique.as_ref().and_then(|u| u.base.clone()),
        }
    }
    /// A virtual item for consultation only, as in createVirtualItem upstream.
    /// It contains no rolls, sockets, item level or copied text.
    pub fn virtual_item(&self) -> Result<Item, String> {
        let entry = catalogue().get(self.id).ok_or("Identité inconnue.")?;
        if Self::from_index(self.id) != *self
            || !["GEM", "UNIQUE"].contains(&entry.namespace.as_str())
        {
            return Err("L’identité ne correspond pas au catalogue.".into());
        }
        let gem = entry.namespace == "GEM";
        Ok(Item {
            class: "Catalogue (objet virtuel)".into(),
            rarity: if gem { Rarity::Gem } else { Rarity::Unique },
            name: entry.name.clone(),
            base: self.base.clone().unwrap_or_else(|| entry.name.clone()),
            item_level: None,
            gem_level: gem.then_some(1),
            quality: None,
            map_tier: None,
            links: 0,
            corrupted: false,
            mirrored: false,
            unmodifiable: false,
            unidentified: false,
            raw: "VIRTUAL_ITEM".into(),
            modifiers: Vec::new(),
            namespace: entry.namespace.clone(),
            trade_tag: entry.trade_tag.clone(),
            trade_discriminator: entry.trade_discriminator.clone(),
            category: gem.then(|| "Gem".into()),
            stack_size: None,
            stack_max: None,
            influences: Vec::new(),
            properties: Default::default(),
            armour_base: Default::default(),
            reference: entry.reference.clone(),
            unknown_modifiers: Vec::new(),
            map_area: None,
            unique_dust_value: entry.unique.as_ref().and_then(|u| u.dust),
            area: AreaProperties::default(),
            logbook_areas: Vec::new(),
            mercenary: None,
            market: MarketMetadata {
                known: true,
                name: entry.name.clone(),
                craftable: entry.craftable.is_some(),
                exchangeable: entry.exchangeable,
                unique_base: None,
                gem: entry.gem.clone(),
                fixed_stats: entry.unique.as_ref().and_then(|u| u.fixed_stats.clone()),
                ..Default::default()
            },
        })
    }
    /// Add only the known base identity omitted by the upstream virtual unique.
    /// Its original query is rejected by Trade as an unknown item base type.
    pub fn trade_item(&self) -> Result<Item, String> {
        let mut item = self.virtual_item()?;
        if let Some(base) = &self.base {
            if !catalogue()
                .iter()
                .any(|e| e.namespace == "ITEM" && e.reference == *base)
            {
                return Err("Base de l’unique absente du catalogue.".into());
            }
            item.market.unique_base = Some(base.clone());
        }
        Ok(item)
    }
}
fn js_space(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
fn candidates(kind: Kind) -> impl Iterator<Item = &'static super::CatalogueEntry> {
    // Keep the exact upstream line-substring predicate, including catalogue
    // ordering and duplicate names. The current asset has no old alt-quality gems.
    static GEM_ENTRIES: std::sync::OnceLock<Vec<bool>> = std::sync::OnceLock::new();
    let gem_entries = GEM_ENTRIES.get_or_init(|| {
        include_str!("../../../renderer/public/data/en/items.ndjson")
            .lines()
            .map(|l| l.contains("altQuality\":[\"Anomalous"))
            .collect()
    });
    catalogue()
        .iter()
        .enumerate()
        .filter(move |(index, entry)| match kind {
            Kind::Gem => gem_entries[*index],
            Kind::Replica => entry.reference.starts_with("Replica"),
        })
        .map(|(_, entry)| entry)
}

pub fn candidate_count(kind: Kind) -> usize {
    candidates(kind).count()
}
/// None means upstream's `false`: fewer than three UTF-16 code units or more
/// than five matches. Some(empty) means no matching item.
pub fn find(search: &str, kind: Kind) -> Option<Vec<Choice>> {
    let search = search.trim_matches(js_space);
    if search.encode_utf16().count() < 3 {
        return None;
    }
    let lower = search.to_lowercase();
    let mut words: Vec<_> = lower.split(js_space).filter(|s| !s.is_empty()).collect();
    words.sort_by_key(|s| std::cmp::Reverse(s.encode_utf16().count()));
    let namespace = if kind == Kind::Gem { "GEM" } else { "UNIQUE" };
    let mut out = Vec::new();
    for candidate in candidates(kind) {
        let name = candidate.name.to_lowercase();
        if words.iter().all(|word| name.contains(word))
            && name.split(js_space).any(|part| part.starts_with(words[0]))
        {
            for (id, entry) in catalogue()
                .iter()
                .enumerate()
                .filter(|(_, e)| e.namespace == namespace && e.name == candidate.name)
            {
                out.push(Choice::from_index(id));
                if out.len() > 5 {
                    return None;
                }
                if entry.unique.is_none() && entry.discriminator.is_none() {
                    break;
                }
            }
        }
    }
    Some(out)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selected {
    pub choice: Choice,
    pub chaos: Option<f64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Basket {
    pub items: Vec<Selected>,
}
impl Basket {
    pub fn add(&mut self, choice: Choice, chaos: Option<f64>) -> bool {
        if self.items.iter().any(|i| i.choice.name == choice.name) {
            return false;
        }
        let item = Selected {
            choice,
            chaos: chaos.filter(|v| v.is_finite() && *v >= 0.),
        };
        if self.items.len() < 5 {
            self.items.push(item);
            self.sort();
        } else {
            self.items = vec![item];
        }
        true
    }
    pub fn sort(&mut self) {
        self.items
            .sort_by(|a, b| b.chaos.unwrap_or(0.).total_cmp(&a.chaos.unwrap_or(0.)));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_search_limits_and_longest_word_prefix_are_preserved() {
        assert!(find("", Kind::Replica).is_none());
        assert!(find(" re ", Kind::Replica).is_none());
        assert!(find("replica", Kind::Replica).is_none());
        assert_eq!(find("replica zzz", Kind::Replica), Some(vec![]));
        assert!(find("eadhunter", Kind::Replica).unwrap().is_empty());
        assert_eq!(
            find("  HEADHUNTER\tReplica\u{feff}", Kind::Replica).unwrap()[0].reference,
            "Replica Headhunter"
        );
        assert_eq!(candidate_count(Kind::Gem), 0);
        assert!(find("Fireball", Kind::Gem).unwrap().is_empty());
    }
    #[test]
    fn virtual_consultation_never_invents_properties_or_copied_text() {
        let choice = find("Replica Headhunter", Kind::Replica).unwrap().remove(0);
        let item = choice.virtual_item().unwrap();
        assert_eq!(item.raw, "VIRTUAL_ITEM");
        assert_eq!(item.item_level, None);
        assert!(item.modifiers.is_empty());
        assert_eq!(item.links, 0);
        assert_eq!(item.market.unique_base, None);
        assert_eq!(
            choice.trade_item().unwrap().market.unique_base.as_deref(),
            Some("Leather Belt")
        );
        assert_eq!(item.category, None);
        let mut forged = choice;
        forged.name = "Mageblood".into();
        assert!(forged.virtual_item().is_err());
    }
    #[test]
    fn basket_deduplicates_orders_and_replaces_on_sixth_selection() {
        let names = [
            "Headhunter",
            "Shroud",
            "Kaom's Heart",
            "Hyrri's Ire",
            "Lioneye's Paws",
            "Dreamfeather",
        ];
        let mut basket = Basket::default();
        for (index, name) in names.iter().enumerate() {
            let c = find(&format!("Replica {name}"), Kind::Replica)
                .unwrap()
                .remove(0);
            assert!(basket.add(c.clone(), Some(index as f64)));
            assert!(!basket.add(c, Some(999.)));
            if index < 5 {
                assert_eq!(basket.items[0].chaos, Some(index as f64));
            }
        }
        assert_eq!(basket.items.len(), 1);
        assert_eq!(basket.items[0].choice.reference, "Replica Dreamfeather");
    }
}
