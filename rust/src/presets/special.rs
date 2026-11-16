//! Rules exported from upstream atzoatl-rules.ts and reflection-rules.ts.
use crate::{calculated::Candidate, item::Item};
use serde::Deserialize;
use std::sync::OnceLock;
#[derive(Deserialize)]
struct Rule {
    #[serde(rename = "ref")]
    reference: String,
    disabled: Option<bool>,
    #[serde(default)]
    remove: bool,
    hidden: Option<String>,
    #[serde(default, rename = "explosivesRoom")]
    explosives: bool,
    difficulty: Option<String>,
}
#[derive(Deserialize)]
struct Rules {
    atzoatl: Vec<Rule>,
    reflections: Vec<Rule>,
}
fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| {
        serde_json::from_str(include_str!("../data/special-rules.json"))
            .expect("upstream special rules")
    })
}
pub(super) fn apply(item: &Item, filters: &mut Vec<Candidate>) -> bool {
    if item.reference == "Chronicle of Atzoatl" {
        for f in filters.iter_mut() {
            f.disabled = true;
        }
        let explosives = rules().atzoatl.iter().filter(|r| r.explosives).any(|r| {
            filters
                .iter()
                .any(|f| f.modifier.reference == r.reference && f.modifier.value == Some(1.))
        });
        filters.retain_mut(|f| {
            let Some(rule) = rules()
                .atzoatl
                .iter()
                .find(|r| r.reference == f.modifier.reference)
            else {
                return true;
            };
            let obstructed = f.modifier.value == Some(2.);
            if rule.remove || (obstructed && !explosives) {
                return false;
            }
            if let Some(hidden) = &rule.hidden {
                f.hidden = Some(hidden.clone());
            }
            if let Some(disabled) = rule.disabled {
                f.disabled = disabled;
            }
            if obstructed {
                // The explosives open the room: accept either Trade option.
                f.modifier.value = None;
                f.display_value = None;
            }
            true
        });
        true
    } else if item.reference == "Mirrored Tablet" {
        filters.retain_mut(|f| {
            if let Some(rule) = rules()
                .reflections
                .iter()
                .find(|r| r.reference == f.modifier.reference)
            {
                if rule.difficulty.as_deref() == Some("any") {
                    f.modifier.value = None;
                    f.display_value = None;
                    f.initial_min = None;
                    f.initial_max = None;
                    f.default_min = None;
                    f.default_max = None;
                }
                if let Some(disabled) = rule.disabled {
                    f.disabled = disabled;
                }
            } else {
                if f.display_value.is_none_or(|v| v < 8.) {
                    return false;
                }
                f.hidden = Some("low_tier_reflection".into());
            }
            true
        });
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        filters::{self, Filters, StatFilter},
        presets::{self, Mode},
    };
    fn query(item: &Item, mode: Mode) -> serde_json::Value {
        let preset = presets::create(item, 10., mode);
        filters::query(
            item,
            &Filters {
                stats: preset
                    .filters
                    .iter()
                    .filter(|f| !f.disabled)
                    .map(StatFilter::from_candidate)
                    .collect(),
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn inaccessible_corruption_room_needs_accessible_explosives() {
        let raw = include_str!("../fixtures/synthetic-atzoatl.txt");
        let temple = Item::parse(raw).unwrap();
        let q = query(&temple, Mode::Exact);
        let corruption = q["query"]["stats"][0]["filters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["id"] == "pseudo.pseudo_temple_corruption_room_3")
            .unwrap();
        assert_eq!(corruption["value"], serde_json::json!({}));
        let no_explosives = Item::parse(&raw.replace("Explosives Room (Tier 1)\n", "")).unwrap();
        let q = query(&no_explosives, Mode::Exact);
        assert!(q["query"]["stats"][0]["filters"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["id"] != "pseudo.pseudo_temple_corruption_room_3"));
        let p = presets::create(&temple, 10., Mode::Exact);
        assert!(p
            .filters
            .iter()
            .all(|f| !f.modifier.reference.contains("Explosives")));
    }
    #[test]
    fn logbook_zone_selection_cannot_mix_faction_or_boss_filters() {
        let item = Item::parse(include_str!("../fixtures/synthetic-logbook.txt")).unwrap();
        assert_eq!(
            presets::available(&item),
            vec![Mode::Logbook(0), Mode::Logbook(1), Mode::Logbook(2)]
        );
        let first = query(&item, Mode::Logbook(0));
        assert_eq!(
            first["query"]["stats"][0]["filters"][0]["id"],
            "pseudo.pseudo_logbook_faction_mercenaries"
        );
        assert_eq!(
            first["query"]["stats"][0]["filters"][1]["id"],
            "implicit.stat_3159649981|2"
        );
        let second = query(&item, Mode::Logbook(1));
        assert_eq!(
            second["query"]["stats"][0]["filters"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            second["query"]["stats"][0]["filters"][0]["id"],
            "pseudo.pseudo_logbook_faction_druids"
        );
        let third = query(&item, Mode::Logbook(2));
        assert_eq!(
            third["query"]["stats"][0]["filters"][1]["id"],
            "implicit.stat_3159649981|4"
        );
    }
    #[test]
    fn repeated_reflections_use_maximum_and_delirium_has_no_difficulty_constraint() {
        let item = Item::parse(include_str!("../fixtures/synthetic-mirrored-tablet.txt")).unwrap();
        let p = presets::create(&item, 10., Mode::Exact);
        let paradise = p
            .filters
            .iter()
            .find(|f| f.modifier.reference.starts_with("Reflection of Paradise"))
            .unwrap();
        assert_eq!(paradise.display_value, Some(12.));
        assert!(!paradise.disabled);
        let delirium = p
            .filters
            .iter()
            .find(|f| f.modifier.reference.starts_with("Reflection of Delirium"))
            .unwrap();
        assert_eq!(delirium.initial_min, None);
        assert_eq!(delirium.display_value, None);
        let chasm = p
            .filters
            .iter()
            .find(|f| f.modifier.reference.starts_with("Reflection of the Chasm"))
            .unwrap();
        assert_eq!(chasm.display_value, Some(9.));
        assert_eq!(chasm.hidden.as_deref(), Some("low_tier_reflection"));
        assert!(chasm.disabled);
        let q = query(&item, Mode::Exact);
        assert_eq!(
            q["query"]["stats"][0]["filters"].as_array().unwrap().len(),
            3
        );
    }
}

#[cfg(test)]
mod incomplete_clipboard_tests {
    use crate::{
        filters::{self, Filters},
        item::Item,
        presets,
    };
    #[test]
    fn logbook_without_areas_cannot_silently_search_every_faction() {
        let item=Item::parse("Item Class: Expedition Logbooks\nRarity: Normal\nExpedition Logbook\n--------\nArea Level: 83").unwrap();
        assert!(presets::available(&item).is_empty());
        assert!(filters::search_query(&item, &Filters::default())
            .unwrap_err()
            .contains("Zones du logbook"));
    }
}
