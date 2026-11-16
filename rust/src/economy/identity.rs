//! Direct port of trends/getDetailsId.ts. All names are catalogue references.
use crate::item::{Item, Rarity};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key {
    pub namespace: String,
    pub name: String,
    pub variant: Option<String>,
}
impl Key {
    pub fn for_item(item: &Item) -> Option<Self> {
        let variant = if item.category.as_deref() == Some("Gem") {
            let level = item.gem_level?;
            let special = ["Empower Support", "Enlighten Support", "Enhance Support"]
                .contains(&item.reference.as_str());
            let exact = special
                || ["Portal", "Brand Recall", "Blood and Sand"].contains(&item.reference.as_str())
                || level >= 20;
            let mut v = if exact { level } else { 1 }.to_string();
            if let Some(q) = item.quality.filter(|q| {
                *q > 0 && !special && !(item.reference == "Brand Recall" && item.corrupted)
            }) {
                v += &format!("/{}", if (16..=20).contains(&q) { 20 } else { q });
            }
            if item.corrupted && item.reference != "Portal" {
                v.push('c');
            }
            Some(v)
        } else if item.category.as_deref() == Some("Map") {
            Some(if item.rarity == Rarity::Unique {
                "T0".into()
            } else {
                format!(
                    "T{}, {}",
                    item.map_tier.unwrap_or(0),
                    if item.map_tier.is_some_and(|v| v != 0) {
                        "Gen-24"
                    } else {
                        "Atlas"
                    }
                )
            })
        } else if item.rarity == Rarity::Unique {
            let base = item.market.unique_base.as_ref()?;
            let mut parts = Vec::new();
            if let Some(variant) = unique_variant(item)? {
                parts.push(variant);
            }
            if item.category.as_deref() != Some("Flask") {
                parts.push(if item.category.as_deref() == Some("Sanctum Relic") {
                    "Relic".into()
                } else {
                    base.clone()
                });
            }
            if let Some(links) = item.market.linked_sockets {
                parts.push(format!("{links}L"));
            }
            (!parts.is_empty()).then(|| parts.join(", "))
        } else if valuable_base(item) {
            if item.influences.len() > 1 {
                return None;
            }
            let level = item.item_level?;
            let mut v = if level >= 86 {
                "86+".into()
            } else {
                level.to_string()
            };
            if let Some(influence) = item.influences.first() {
                v += &format!(", {influence}");
            }
            Some(v)
        } else if item.reference == "Filled Coffin" {
            let modifier = item.modifiers.first()?;
            let value = modifier
                .sources
                .first()
                .and_then(|s| s.value)
                .or(modifier.value)?;
            let level = item.item_level?;
            return Some(Self {
                namespace: item.namespace.clone(),
                name: modifier.reference.replacen('#', &value.to_string(), 1),
                variant: Some(
                    [1, 70, 80, 84]
                        .into_iter()
                        .rfind(|n| *n <= level)
                        .unwrap_or(1)
                        .to_string(),
                ),
            });
        } else {
            None
        };
        Some(Self {
            namespace: item.namespace.clone(),
            name: item.reference.clone(),
            variant,
        })
    }
    pub fn from_query_id(id: &str) -> Option<Self> {
        let (namespace, name) = id.split_once("::")?;
        let (name, variant) = name
            .split_once(" // ")
            .map(|(a, b)| (a, Some(b.to_string())))
            .unwrap_or((name, None));
        Some(Self {
            namespace: namespace.into(),
            name: name.into(),
            variant,
        })
    }
    pub fn query_id(&self) -> String {
        format!(
            "{}::{}{}",
            self.namespace,
            self.name,
            self.variant
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| format!(" // {v}"))
                .unwrap_or_default()
        )
    }
    pub fn slug(&self) -> String {
        let text = if let Some(v) = self.variant.as_ref().filter(|v| !v.is_empty()) {
            format!("{}, {v}", self.name)
        } else {
            self.name.clone()
        };
        text.nfkd()
            .filter(|c| c.is_ascii_alphanumeric() || [':', '-', ' '].contains(c))
            .map(|c| {
                if c == ' ' {
                    '-'
                } else {
                    c.to_ascii_lowercase()
                }
            })
            .collect()
    }
}
pub fn valuable_base(item: &Item) -> bool {
    [Rarity::Normal, Rarity::Magic, Rarity::Rare].contains(&item.rarity)
        && (crate::properties::is_armour(item.category.as_deref())
            || crate::properties::is_weapon(item.category.as_deref())
            || [Some("Ring"), Some("Belt"), Some("Amulet"), Some("Quiver")]
                .contains(&item.category.as_deref()))
}
fn unique_variant(item: &Item) -> Option<Option<String>> {
    let has = |reference: &str| item.modifiers.iter().any(|m| m.reference == reference);
    let number = |reference: &str| {
        item.modifiers
            .iter()
            .find(|m| m.reference == reference)
            .and_then(|m| m.sources.first().and_then(|s| s.value).or(m.value))
    };
    let variant=match item.reference.as_str() {
        "Vessel of Vinktar"=>[
            ("Adds # to # Lightning Damage to Attacks during Flask effect","Added Attacks"),
            ("Adds # to # Lightning Damage to Spells during Flask effect","Added Spells"),
            ("Damage Penetrates #% Lightning Resistance during Flask effect","Penetration"),
            ("#% of Physical Damage Converted to Lightning during Flask effect","Conversion")
        ].into_iter().find(|(r,_)|has(r)).map(|(_,v)|v),
        "Atziri's Splendour"=>[
            (has("#% increased Armour, Evasion and Energy Shield"),"Armour/Evasion/ES"),
            (has("#% increased Evasion and Energy Shield")&&has("+# to maximum Energy Shield"),"Evasion/ES"),
            (has("#% increased Evasion and Energy Shield")&&has("+# to maximum Life"),"Evasion/ES/Life"),
            (has("#% increased Armour and Energy Shield")&&has("+# to maximum Energy Shield"),"Armour/ES"),
            (has("#% increased Armour and Energy Shield")&&has("+# to maximum Life"),"Armour/ES/Life"),
            (has("#% increased Armour and Evasion")&&has("+# to maximum Life"),"Armour/Evasion"),
            (has("+# to maximum Energy Shield")&&has("#% increased Energy Shield"),"ES"),
            (has("#% increased Evasion Rating")&&has("+# to maximum Life"),"Evasion"),
            (has("#% increased Armour")&&has("+# to maximum Life"),"Armour")
        ].into_iter().find(|(yes,_)|*yes).map(|(_,v)|v),
        "Bubonic Trail"|"Lightpoacher"|"Shroud of the Lightless"|"Tombfist"=>match number("Has # Abyssal Sockets")? {2.=>Some("2 Jewels"),1.=>Some("1 Jewel"),_=>None},
        "Volkuur's Guidance"=>[
            ("Adds # to # Cold Damage to Spells and Attacks","Cold"),
            ("Adds # to # Fire Damage to Spells and Attacks","Fire"),
            ("Adds # to # Lightning Damage to Spells and Attacks","Lightning")
        ].into_iter().find(|(r,_)|has(r)).map(|(_,v)|v),
        "Yriel's Fostering"=>[
            ("Projectiles from Attacks have #% chance to Maim on Hit while\nyou have a Bestial Minion","Maim"),
            ("Projectiles from Attacks have #% chance to Poison on Hit while\nyou have a Bestial Minion","Poison"),
            ("Projectiles from Attacks have #% chance to inflict Bleeding on Hit while\nyou have a Bestial Minion","Bleeding")
        ].into_iter().find(|(r,_)|has(r)).map(|(_,v)|v),
        "Doryani's Invitation"=>[
            ("#% increased Global Physical Damage","Physical"),("#% increased Fire Damage","Fire"),("#% increased Cold Damage","Cold"),("#% increased Lightning Damage","Lightning")
        ].into_iter().find(|(r,_)|has(r)).map(|(_,v)|v),
        "Impresence"=>[
            ("Adds # to # Cold Damage","Cold"),("Adds # to # Chaos Damage","Chaos"),("Adds # to # Fire Damage","Fire"),("Adds # to # Lightning Damage","Lightning"),("Adds # to # Physical Damage","Physical")
        ].into_iter().find(|(r,_)|has(r)).map(|(_,v)|v),
        "Voices"=>match number("Adds # Small Passive Skills which grant nothing")? {7.=>Some("7 passives"),5.=>Some("5 passives"),3.=>Some("3 passives"),_=>None},
        "The First Crest"=>[
            ("Aureus Coins are converted to Experience upon defeating the Herald of the Scourge","Experience"),
            ("Aureus Coins are converted to Relics upon defeating the Herald of the Scourge","Relics"),
            ("Aureus Coins are converted to Tainted Currency upon defeating the Herald of the Scourge","Tainted Currency")
        ].into_iter().find(|(r,_)|has(r)).map(|(_,v)|v),
        _=>None,
    };
    Some(variant.map(str::to_string))
}
