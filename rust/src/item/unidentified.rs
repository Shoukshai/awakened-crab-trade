//! UnidentifiedResolver.vue and Parser.makeIdentifiedUnique, without game input.
use super::{catalogue, Item, Rarity};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Choice {
    pub reference: String,
    pub name: String,
}
impl Item {
    pub fn needs_unique_identity(&self) -> bool {
        self.rarity == Rarity::Unique && self.unidentified && self.market.unique_base.is_none()
    }
    /// Catalogue order, one entry per reference; variants follow upstream's
    /// first-match policy. No item level or hidden modifier is inferred.
    pub fn unique_identity_choices(&self) -> Vec<Choice> {
        if !self.needs_unique_identity() {
            return Vec::new();
        }
        let mut seen = HashSet::new();
        catalogue()
            .iter()
            .filter(|e| {
                e.namespace == "UNIQUE"
                    && e.unique.as_ref().and_then(|u| u.base.as_deref())
                        == Some(self.reference.as_str())
                    && seen.insert(e.reference.as_str())
            })
            .map(|e| Choice {
                reference: e.reference.clone(),
                name: e.name.clone(),
            })
            .collect()
    }
    /// This selects an identity for consultation. It does not identify the
    /// actual item: raw text, unidentified flag, properties and mods are kept.
    pub fn with_unique_identity(&self, reference: &str) -> Result<Self, String> {
        if !self.needs_unique_identity() {
            return Err("Cet objet n’attend pas de choix d’identité unique.".into());
        }
        let entry = catalogue()
            .iter()
            .find(|e| {
                e.namespace == "UNIQUE"
                    && e.reference == reference
                    && e.unique.as_ref().and_then(|u| u.base.as_deref())
                        == Some(self.reference.as_str())
            })
            .ok_or("Cette identité ne correspond pas à la base copiée.")?;
        let mut item = self.clone();
        item.namespace = entry.namespace.clone();
        item.reference = entry.reference.clone();
        item.name = entry.name.clone();
        item.trade_tag = entry.trade_tag.clone();
        item.trade_discriminator = entry.trade_discriminator.clone();
        item.unique_dust_value = entry.unique.as_ref().and_then(|u| u.dust);
        item.market.name = entry.name.clone();
        item.market.known = true;
        item.market.craftable = entry.craftable.is_some();
        item.market.exchangeable = entry.exchangeable;
        item.market.unique_base = entry.unique.as_ref().and_then(|u| u.base.clone());
        item.market.fixed_stats = entry.unique.as_ref().and_then(|u| u.fixed_stats.clone());
        item.market.gem = entry.gem.clone();
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(base: &str) -> Item {
        Item::parse(&format!("Item Class: Synthetic Fixtures\nRarity: Unique\n{base}\n--------\nItem Level: 84\n--------\nUnidentified\n--------\nCorrupted\n")).unwrap()
    }
    #[test]
    fn choices_are_ordered_deduplicated_and_limited_to_the_base() {
        let i = item("Heavy Belt");
        let choices = i.unique_identity_choices();
        assert_eq!(choices.len(), 12);
        assert_eq!(choices[0].reference, "Belt of the Deceiver");
        assert_eq!(
            choices
                .iter()
                .filter(|c| c.reference == "Doryani's Invitation")
                .count(),
            1
        );
        assert!(choices.iter().any(|c| c.reference == "Mageblood"));
        assert!(!choices.iter().any(|c| c.reference == "Headhunter"));
        assert!(i.with_unique_identity("Headhunter").is_err());
        assert!(item("Unknown Base").unique_identity_choices().is_empty());
    }
    #[test]
    fn choosing_preserves_the_actual_copy_and_unidentified_status() {
        let i = item("Prismatic Jewel");
        let chosen = i.with_unique_identity("Watcher's Eye").unwrap();
        assert!(chosen.unidentified && chosen.corrupted);
        assert_eq!(chosen.raw, i.raw);
        assert_eq!(chosen.base, i.base);
        assert_eq!(chosen.item_level, i.item_level);
        assert_eq!(chosen.category, i.category);
        assert_eq!(
            serde_json::to_value(&chosen.modifiers).unwrap(),
            serde_json::to_value(&i.modifiers).unwrap()
        );
        assert_eq!(chosen.namespace, "UNIQUE");
        assert_eq!(
            chosen.market.unique_base.as_deref(),
            Some("Prismatic Jewel")
        );
        assert!(!chosen.needs_unique_identity());
        assert!(chosen.with_unique_identity("The Anima Stone").is_err());
    }
    #[test]
    fn only_one_identity_does_not_invent_mods_or_apply_to_other_rarities() {
        let i = item("Driftwood Wand");
        assert_eq!(
            i.unique_identity_choices()
                .iter()
                .map(|c| c.reference.as_str())
                .collect::<Vec<_>>(),
            ["Lifesprig", "Splinter of the Moon"]
        );
        let chosen = i.with_unique_identity("Lifesprig").unwrap();
        assert!(chosen.modifiers.is_empty());
        let normal = Item::parse(include_str!("../fixtures/driftwood-wand.txt")).unwrap();
        assert!(!normal.needs_unique_identity());
        assert!(normal.unique_identity_choices().is_empty());
        assert!(normal.with_unique_identity("Lifesprig").is_err());
    }
}
