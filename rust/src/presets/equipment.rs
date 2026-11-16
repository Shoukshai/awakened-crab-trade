//! Equipment selection and final filter tweaks from create-stat-filters.ts.
use crate::{
    calculated::Candidate,
    item::{Item, Rarity},
    modifiers::{self, translation, Modifier},
    properties,
};
pub fn has_crafting_value(item: &Item) -> bool {
    item.market.synthesised
        || item.market.fractured
        || !item.influences.is_empty()
        || item.category.as_deref() == Some("Cluster Jewel")
        || (item.category.as_deref() == Some("Jewel") && item.rarity == Rarity::Magic)
        || (![Some("Jewel"), Some("Abyss Jewel")].contains(&item.category.as_deref())
            && item.item_level.is_some_and(|v| v >= 82))
}
pub fn likely_finished(item: &Item) -> bool {
    item.rarity == Rarity::Unique
        || item.modifiers.iter().any(|m| m.kind == "crafted")
        || (item.quality == Some(20) && item.properties.memory_strands.is_none())
        || item.corrupted
        || item.mirrored
}
pub fn pseudo(item: &Item, range: f64) -> Vec<Candidate> {
    let mut working = item.clone();
    for m in &mut working.modifiers {
        if m.kind == "fractured" {
            if let Some(explicit) = translation::retype(m, "explicit", item.category.as_deref()) {
                *m = explicit;
            }
        }
    }
    let props = properties::equipment(&working, range);
    let mut filters: Vec<_> = props
        .filters
        .into_iter()
        .filter(|f| f.modifier.id != "item.base_percentile" || item.rarity == Rarity::Unique)
        .collect();
    let pseudo = crate::pseudo::apply(&working, &props.remaining, range);
    filters.extend(pseudo.filters);
    let mut remaining = pseudo.remaining;
    let mut groups = Vec::new();
    if item.market.unique_base.as_deref() == Some("Timeless Jewel") {
        if let Some(index) = remaining
            .iter()
            .position(|m| m.better == 0 && !translation::metadata(m).family.is_empty())
        {
            let original = remaining.remove(index);
            for reference in translation::metadata(&original).family {
                let mut m = if reference == original.reference {
                    original.clone()
                } else {
                    let Some(mut other) = translation::exclusion(
                        &reference,
                        &original.kind,
                        item.category.as_deref(),
                        false,
                    ) else {
                        continue;
                    };
                    other.value = original.value;
                    other.min = original.min;
                    other.max = original.max;
                    other.sources = original.sources.clone();
                    other
                };
                let is_original = reference == original.reference;
                m.source_count = original.source_count;
                let mut c = Candidate::new(m, item, range, !is_original);
                c.disabled = !is_original;
                if !is_original {
                    c.tag = "pseudo".into();
                }
                if !is_original {
                    c.modifier.text = c.display_reference.replacen(
                        '#',
                        &c.display_value.unwrap_or(0.).to_string(),
                        1,
                    );
                }
                c.group = Some("timeless_keystone".into());
                groups.push(c);
            }
        }
    }
    if !item.corrupted && !item.mirrored {
        remaining.retain(|m| m.kind != "fractured");
        remaining.extend(
            item.modifiers
                .iter()
                .filter(|m| m.kind == "fractured")
                .cloned(),
        );
    }
    if item.market.veiled {
        remaining.retain(|m| m.kind != "veiled");
    }
    filters.extend(
        remaining
            .into_iter()
            .map(|m| Candidate::new(m, item, range, true)),
    );
    if item.market.veiled {
        for f in &mut filters {
            f.disabled = true;
        }
    }
    final_tweaks(item, &mut filters);
    filters.extend(groups);
    filters
}
pub fn cluster(filters: &mut [Candidate], exact: bool) {
    for f in filters {
        if f.modifier.reference == "# Added Passive Skills are Jewel Sockets" {
            f.hidden = Some("filters.hide_const_roll".into());
            f.disabled = true;
        }
        if f.modifier.reference == "Adds # Passive Skills" {
            f.disabled = false;
            match f.display_value {
                Some(4.) => f.initial_max = Some(5.),
                Some(5.) if exact => f.initial_min = f.default_min,
                Some(3. | 6. | 10. | 11. | 12.) => {
                    f.initial_min = f.default_min;
                    f.initial_max = None;
                }
                _ => {}
            }
        }
    }
}
fn property_sources(item: &Item, id: &str) -> Vec<modifiers::SourceInfo> {
    let (flat, incr): (&[&str], &[&str]) = match id {
        "item.armour" => (&["+# to Armour"], properties::ARMOUR_INCR),
        "item.evasion_rating" => (&["+# to Evasion Rating"], properties::EVASION_INCR),
        "item.energy_shield" => (&["+# to maximum Energy Shield"], properties::ES_INCR),
        "item.ward" => (&["+# to Ward"], &["#% increased Ward"]),
        "item.block" => (&["+#% Chance to Block"], &[]),
        "item.aps" => (&[], &["#% increased Attack Speed"]),
        "item.crit" => (&[], &["#% increased Critical Strike Chance"]),
        _ => (&[], &[]),
    };
    item.modifiers
        .iter()
        .filter(|m| flat.contains(&m.reference.as_str()) || incr.contains(&m.reference.as_str()))
        .flat_map(|m| m.sources.clone())
        .collect()
}
fn hide_constant(item: &Item, f: &mut Candidate) {
    if item.rarity != Rarity::Unique || !["explicit", "property"].contains(&f.tag.as_str()) {
        return;
    }
    let sources = if f.tag == "property" {
        property_sources(item, &f.modifier.id)
    } else {
        f.modifier.sources.clone()
    };
    if item.quality.is_some_and(|q| q != 0)
        && [Some("Ring"), Some("Belt"), Some("Amulet")].contains(&item.category.as_deref())
        && sources.iter().any(|s| s.roll_incr && s.scalable)
    {
        return;
    }
    if item.corrupted && sources.iter().any(|s| s.scalable) {
        let volatile = item
            .modifiers
            .iter()
            .any(|m| m.sources.iter().any(|s| s.legacy && s.scalable));
        if !volatile {
            f.disabled = false;
        }
        return;
    }
    let bounds = f
        .modifier
        .min
        .zip(f.modifier.max)
        .is_some_and(|(a, b)| a != b)
        && f.modifier.better != 0;
    if f.display_value.is_none() || !bounds {
        f.initial_min = None;
        f.initial_max = None;
        f.hidden = Some("filters.hide_const_roll".into());
        f.disabled = true;
    } else if [
        "item.armour",
        "item.evasion_rating",
        "item.energy_shield",
        "item.ward",
    ]
    .contains(&f.modifier.id.as_str())
        && sources.iter().all(|s| !s.variable)
        && properties::quality(item).0 < 21.
    {
        f.initial_min = None;
        f.initial_max = None;
        f.hidden = Some("hide_variable_by_base_percentile_only".into());
        f.disabled = true;
    }
    if item.market.foulborn && f.tag == "explicit" {
        f.disabled = false;
    }
}
fn empty_modifier(item: &Item) -> Option<u32> {
    if item.rarity != Rarity::Rare || item.corrupted || item.mirrored {
        return None;
    }
    let mods = &item.market.random_mods;
    let craft = mods.iter().find(|m| m.kind == "crafted");
    if (mods.len() == 5 && craft.is_none()) || (mods.len() == 6 && craft.is_some()) {
        for (generation, option) in [("prefix", 1), ("suffix", 2)] {
            let count = mods
                .iter()
                .filter(|m| m.generation.as_deref() == Some(generation))
                .count();
            let count = count
                - usize::from(craft.is_some_and(|m| m.generation.as_deref() == Some(generation)));
            if count == 2 {
                return Some(option);
            }
        }
    }
    None
}
fn final_tweaks(item: &Item, filters: &mut Vec<Candidate>) {
    if item.category.as_deref() == Some("Cluster Jewel") && item.rarity != Rarity::Unique {
        cluster(filters, false);
    }
    if item.category.as_deref() == Some("Flask") {
        super::flask_rules(item, filters);
    }
    if let Some(option) = empty_modifier(item) {
        let m = Modifier {
            text: "1 Empty or Crafted Modifier".into(),
            reference: "1 Empty or Crafted Modifier".into(),
            id: "item.has_empty_modifier".into(),
            ids: vec!["item.has_empty_modifier".into()],
            kind: "pseudo".into(),
            value: Some(option as f64),
            min: None,
            max: None,
            decimal: false,
            legacy: false,
            unscalable: false,
            option: true,
            inverted: false,
            better: 0,
            source_count: 0,
            fractured_t1: false,
            tier: None,
            sources: vec![],
        };
        let mut c = Candidate::new(m, item, 0., true);
        c.disabled = true;
        c.tag = "pseudo".into();
        c.hidden = Some("filters.hide_empty_mod".into());
        filters.push(c);
    }
    if [Some("Amulet"), Some("Ring")].contains(&item.category.as_deref()) {
        if let Some(f) = filters.iter_mut().find(|f| f.oils.is_some()) {
            if item.market.talisman_tier.is_some() {
                f.disabled = false;
            } else if !item.corrupted
                && !item.mirrored
                && !f
                    .oils
                    .as_ref()
                    .and_then(|o| o.first())
                    .is_some_and(|o| ["Golden Oil", "Silver Oil"].contains(&o.as_str()))
            {
                f.hidden = Some("filters.hide_anointment".into());
                f.disabled = true;
            }
        }
    }
    for f in filters.iter_mut() {
        hide_constant(item, f);
        if f.tag == "enchant" {
            if [
                "Quality does not increase Physical Damage",
                "Quality does not increase Defences",
            ]
            .contains(&f.modifier.reference.as_str())
            {
                f.hidden = Some("hide_enchant_meta_stat".into());
            }
        } else if f.tag == "fractured" {
            if translation::retype(&f.modifier, "explicit", item.category.as_deref()).is_some() {
                f.hidden = Some("filters.hide_for_crafting".into());
            }
        } else if translation::metadata(&f.modifier).jewellery_quality {
            f.hidden = Some("hide_jewellery_quality".into());
        } else if f.tag == "implicit"
            && item.rarity == Rarity::Unique
            && !item.corrupted
            && item.category.as_deref() != Some("Jewel")
        {
            f.hidden = Some("hide_unique_base_implicit".into());
            f.disabled = true;
        } else if ["foulborn", "vestigial", "variant"].contains(&f.tag.as_str()) {
            f.disabled = false;
        }
    }
    if item.rarity == Rarity::Unique {
        let visible_armour = filters.iter().any(|f| {
            f.hidden.is_none()
                && [
                    "item.armour",
                    "item.evasion_rating",
                    "item.energy_shield",
                    "item.ward",
                ]
                .contains(&f.modifier.id.as_str())
        });
        if visible_armour {
            if let Some(f) = filters
                .iter_mut()
                .find(|f| f.modifier.id == "item.base_percentile")
            {
                f.hidden = Some("hide_redundant".into());
                f.disabled = true;
            }
        }
        if filters.iter().filter(|f| f.hidden.is_none()).count() <= 3 {
            enable_all(filters);
        }
    }
}
pub fn enable_all(filters: &mut [Candidate]) {
    for f in filters {
        if f.hidden.is_none() {
            f.disabled = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crafting_base_keeps_fracture_while_finished_item_uses_its_explicit_pseudo() {
        let item=Item::parse("Item Class: Rings\nRarity: Rare\nFixture\nGold Ring\n--------\nItem Level: 83\n--------\n{ Fractured Prefix Modifier \"Healthy\" (Tier: 1) }\n+73(70-79) to maximum Life").unwrap();
        assert_eq!(
            super::super::available(&item),
            vec![super::super::Mode::Calculated, super::super::Mode::Base]
        );
        let general = pseudo(&item, 10.);
        assert!(general
            .iter()
            .any(|f| f.modifier.id == "pseudo.pseudo_total_life"));
        assert!(general.iter().any(|f| f.modifier.kind == "fractured"
            && f.hidden.as_deref() == Some("filters.hide_for_crafting")));
        let base = super::super::create(&item, 10., super::super::Mode::Base);
        assert!(base
            .filters
            .iter()
            .any(|f| !f.disabled && f.modifier.kind == "fractured"));
        assert!(!base
            .filters
            .iter()
            .any(|f| f.modifier.id == "pseudo.pseudo_total_life"));
    }
    #[test]
    fn empty_prefix_is_optional_and_encodes_two_count_constraints() {
        let mut text="Item Class: Rings\nRarity: Rare\nFixture\nGold Ring\n--------\nItem Level: 83\n--------\n".to_string();
        for i in 0..5 {
            text.push_str(&format!(
                "{{ {} Modifier \"Fixture\" }}\n+73(70-79) to maximum Life\n",
                if i < 2 { "Prefix" } else { "Suffix" }
            ));
        }
        let item = Item::parse(&text).unwrap();
        let candidates = pseudo(&item, 10.);
        let f = candidates
            .iter()
            .find(|c| c.modifier.id == "item.has_empty_modifier")
            .unwrap();
        assert!(f.disabled);
        assert_eq!(f.modifier.value, Some(1.));
        let filters = crate::filters::Filters {
            stats: vec![crate::filters::StatFilter::from_candidate(f)],
            ..Default::default()
        };
        let body = crate::filters::search_query(&item, &filters).unwrap();
        let groups = body["query"]["stats"].as_array().unwrap();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[1]["type"], "count");
        assert_eq!(groups[1]["value"], serde_json::json!({"min":1,"max":1}));
        assert!(groups[1]["filters"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| !f["id"].as_str().unwrap().starts_with("item.")));
    }
    #[test]
    fn timeless_choices_form_one_or_group_instead_of_multiple_required_seeds() {
        let text="Item Class: Jewels\nRarity: Unique\nLethal Pride\nTimeless Jewel\n--------\nItem Level: 83\n--------\n{ Modifier }\nCommanded leadership over 10000(10000-18000) warriors under Kaom(Akoya-Rakiata)\nPassives in radius are Conquered by the Karui";
        let item = Item::parse(text).unwrap();
        let mut candidates = pseudo(&item, 10.);
        let group: Vec<_> = candidates
            .iter_mut()
            .filter(|c| c.group.is_some())
            .collect();
        assert!(group.len() > 1);
        let filters = crate::filters::Filters {
            stats: group
                .into_iter()
                .take(2)
                .map(|c| crate::filters::StatFilter::from_candidate(c))
                .collect(),
            ..Default::default()
        };
        let body = crate::filters::search_query(&item, &filters).unwrap();
        assert_eq!(body["query"]["stats"][0]["filters"], serde_json::json!([]));
        assert_eq!(body["query"]["stats"][1]["type"], "count");
        assert_eq!(body["query"]["stats"][1]["value"]["min"], 1);
        assert_eq!(
            body["query"]["stats"][1]["filters"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
