//! Exact presets and specialised rules from create-stat-filters.ts and
//! pseudo/{maps,heist,flasks} and equipment/final selection rules.
#[path = "presets/equipment.rs"]
mod equipment;
use crate::{
    calculated::Candidate,
    item::{Item, Rarity},
    modifiers::{translation, Modifier},
};
use serde::Serialize;
const MAX: f64 = 9_007_199_254_740_991.;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Mode {
    Properties,
    Bulk,
    Exact,
    Calculated,
    Base,
}
impl Mode {
    pub fn id(self) -> &'static str {
        match self {
            Self::Properties => "properties",
            Self::Bulk => "bulk",
            Self::Exact => "exact",
            Self::Calculated => "calculated",
            Self::Base => "base",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Properties => "Propriétés de zone",
            Self::Bulk => "Cartes en lot (plage ≤ 2 %)",
            Self::Exact => "Objet exact (plage ≤ 2 %)",
            Self::Calculated => "Objet fini · filtres calculés",
            Self::Base => "Base d’artisanat (plage ≤ 2 %)",
        }
    }
    pub fn exact(self) -> bool {
        self != Self::Calculated
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Preset {
    pub mode: Mode,
    pub filters: Vec<Candidate>,
}
fn property(
    item: &Item,
    reference: &str,
    id: &str,
    value: Option<f64>,
    range: f64,
    disabled: bool,
) -> Candidate {
    let modifier = Modifier {
        text: reference.into(),
        reference: reference.into(),
        id: id.into(),
        ids: vec![id.into()],
        kind: "property".into(),
        value,
        min: value.map(|_| 0.),
        max: value.map(|_| MAX),
        decimal: false,
        legacy: false,
        unscalable: false,
        option: false,
        inverted: false,
        better: 1,
        source_count: 0,
        fractured_t1: false,
        tier: None,
        sources: vec![],
    };
    let mut c = Candidate::with_range(modifier, item, range, disabled);
    if id.starts_with("item.heist_job_") {
        c.modifier.text = format!(
            "Métier requis : {} · niveau {}",
            item.area.job.as_deref().unwrap_or("?"),
            value.unwrap_or(0.)
        );
    }
    c
}
fn pseudo(
    item: &Item,
    reference: &str,
    value: f64,
    max: f64,
    range: f64,
    disabled: bool,
) -> Candidate {
    let mut m = translation::pseudo(reference).expect("upstream specialised pseudo stat");
    m.value = Some(value);
    m.min = Some(0.);
    m.max = Some(max);
    Candidate::with_range(m, item, range, disabled)
}
fn exclusion(item: &Item, reference: &str, kind: &str, negate_text: bool) -> Candidate {
    let m = if kind == "pseudo" {
        translation::pseudo(reference)
    } else {
        translation::exclusion(reference, kind, item.category.as_deref(), negate_text)
    }
    .expect("upstream specialised exclusion stat");
    let mut filter = Candidate::with_range(m, item, 0., false);
    filter.exclude = true;
    filter
}
pub fn available(item: &Item) -> Vec<Mode> {
    let category = item.category.as_deref();
    if [Some("Map"), Some("Chart")].contains(&category)
        && item.rarity != Rarity::Unique
        && item.area.completion_reward.is_none()
    {
        if item.rarity == Rarity::Rare && !item.unidentified && !is_blighted(item) {
            if category == Some("Chart") && item.area.special {
                vec![Mode::Bulk, Mode::Properties]
            } else {
                vec![Mode::Properties, Mode::Bulk]
            }
        } else {
            vec![Mode::Bulk]
        }
    } else if [
        Some("Map"),
        Some("Heist Contract"),
        Some("Heist Blueprint"),
        Some("Chart"),
        Some("Invitation"),
        Some("Memory Line"),
        Some("Sentinel"),
        Some("Charm"),
    ]
    .contains(&category)
        || ([
            Some("Flask"),
            Some("Tincture"),
            Some("Sanctum Relic"),
            Some("Idol"),
        ]
        .contains(&category)
            && item.rarity != Rarity::Unique)
        || (!item.market.craftable && item.rarity != Rarity::Unique)
        || item.unidentified
        || item.rarity == Rarity::Normal
    {
        vec![Mode::Exact]
    } else {
        if equipment::likely_finished(item) || !equipment::has_crafting_value(item) {
            vec![Mode::Calculated]
        } else {
            vec![Mode::Calculated, Mode::Base]
        }
    }
}
fn is_blighted(item: &Item) -> bool {
    matches!(
        item.reference.as_str(),
        "Blighted Map" | "Blight-ravaged Map"
    )
}
pub fn create(item: &Item, range: f64, mode: Mode) -> Preset {
    if mode == Mode::Calculated {
        return Preset {
            mode,
            filters: equipment::pseudo(item, range),
        };
    }
    let category = item.category.as_deref();
    let mut filters = Vec::new();
    if is_blighted(item) || category == Some("Invitation") {
        return Preset { mode, filters };
    }
    let range = if mode == Mode::Properties {
        range
    } else {
        range.min(2.)
    };
    filters.extend(
        crate::properties::equipment(item, range)
            .filters
            .into_iter()
            .filter_map(|c| {
                let mut c = Candidate::with_range(c.modifier, item, range, c.disabled);
                if c.modifier.id == "item.base_percentile" {
                    Some(c)
                } else if c.modifier.id == "item.memory_strands" {
                    c.hidden = None;
                    c.disabled = c.display_value.is_some_and(|v| v < 60.);
                    Some(c)
                } else {
                    None
                }
            }),
    );
    let mut keep = vec!["pseudo", "fractured", "enchant", "necropolis", "imbued"];
    if item.influences.is_empty()
        && !item.modifiers.iter().any(|m| m.kind == "fractured")
        && ![Some("Tincture"), Some("Idol"), Some("Chart")].contains(&category)
    {
        keep.push("implicit");
    }
    if item.rarity == Rarity::Magic
        && ![
            Some("Cluster Jewel"),
            Some("Map"),
            Some("Heist Contract"),
            Some("Heist Blueprint"),
            Some("Chart"),
            Some("Sentinel"),
        ]
        .contains(&category)
        || item.rarity == Rarity::Rare && category == Some("Idol")
    {
        keep.push("explicit");
    }
    if category == Some("Flask") {
        keep.push("crafted");
    }
    let a = &item.area;
    if category == Some("Map") && a.completion_reward.is_none() && item.rarity != Rarity::Unique {
        if mode != Mode::Bulk {
            let valuable = a.more_scarabs.unwrap_or(0.) != 0.
                || a.more_currency.unwrap_or(0.) != 0.
                || a.more_div_cards.unwrap_or(0.) != 0.;
            area_props(item, range, valuable, &mut filters);
            for (reference, value, disabled) in [
                ("More Maps: #%", a.more_maps, true),
                ("More Scarabs: #%", a.more_scarabs, false),
                ("More Currency: #%", a.more_currency, false),
                ("More Divination Cards: #%", a.more_div_cards, false),
            ] {
                if let Some(value) = value.filter(|v| *v != 0.) {
                    filters.push(pseudo(item, reference, value, MAX, range, disabled));
                }
            }
        }
        if a.explicit_mods == 8 {
            filters.push(pseudo(item, "# Modifiers", 8., 8., 0., false));
        }
    }
    if category == Some("Chart") && mode != Mode::Bulk {
        area_props(item, range, false, &mut filters);
        if let Some(value) = a.sulphur.filter(|v| *v != 0.) {
            filters.push(property(
                item,
                "Dead Man's Sulphur: +#%",
                "item.chart_sulphur",
                Some(value),
                range,
                false,
            ));
        }
    }
    if a.completion_reward.is_some()
        && !item
            .modifiers
            .iter()
            .any(|m| m.reference == "Players who Die in area are sent to the Void")
    {
        filters.push(exclusion(
            item,
            "Players who Die in area are sent to the Void",
            "explicit",
            false,
        ));
    }
    for m in item
        .modifiers
        .iter()
        .filter(|m| keep.contains(&m.kind.as_str()))
    {
        let variant = m.option || m.better == 0;
        let disabled = if m.kind == "pseudo" {
            true
        } else if m.kind == "explicit"
            && !variant
            && !m.sources.iter().any(|s| s.mechanic.is_some())
        {
            !m.tier.is_some_and(|t| t <= 2)
        } else {
            false
        };
        let mut filter = Candidate::with_range(m.clone(), item, range, disabled);
        if m.reference == "# uses remaining" {
            filter.initial_min = filter.display_value;
        }
        filters.push(filter);
    }
    if category == Some("Cluster Jewel") {
        equipment::cluster(&mut filters, true);
    }
    if [Some("Memory Line"), Some("Sanctum Relic"), Some("Charm")].contains(&category) {
        equipment::enable_all(&mut filters);
    }
    if category == Some("Idol") {
        for f in &mut filters {
            if f.hidden.is_some() {
                continue;
            }
            let m = &f.modifier;
            let goodness = m.value.zip(m.min.zip(m.max)).map(|(v, (min, max))| {
                if min == max {
                    1.
                } else if m.better == -1 {
                    1. - (v - min) / (max - min)
                } else {
                    (v - min) / (max - min)
                }
            });
            if m.better == 0 || goodness.is_none_or(|v| v >= 0.66) {
                f.disabled = false;
            }
        }
    }
    if category == Some("Flask") {
        flask_rules(item, &mut filters);
    }
    if item.rarity != Rarity::Unique && category == Some("Heist Contract") {
        let jobs = [
            ("Lockpicking", "lockpicking"),
            ("Brute Force", "bruteforce"),
            ("Perception", "perception"),
            ("Demolition", "demolition"),
            ("Counter-Thaumaturgy", "counterthaumaturgy"),
            ("Trap Disarmament", "trapdisarmament"),
            ("Agility", "agility"),
            ("Deception", "deception"),
            ("Engineering", "engineering"),
        ];
        if let (Some(job), Some(level)) = (a.job.as_deref(), a.job_level.filter(|n| *n != 0)) {
            if let Some((_, suffix)) = jobs.iter().find(|(name, _)| *name == job) {
                let id = format!("item.heist_job_{suffix}");
                filters.push(property(item, &id, &id, Some(level as f64), 0., false));
            }
        }
        if a.priceless {
            filters.push(property(
                item,
                "Heist Target: Priceless",
                "item.heist_target_priceless",
                None,
                range,
                false,
            ));
        }
    }
    if item.rarity != Rarity::Unique && category == Some("Heist Blueprint") {
        if !filters.iter().any(|c| c.modifier.kind == "enchant") {
            filters.push(exclusion(item, "# Enchant Modifiers", "pseudo", false));
        }
        if let Some(wings) = a.wings_revealed {
            filters.push(property(
                item,
                "Wings Revealed: #",
                "item.heist_wings_revealed",
                Some(wings as f64),
                0.,
                false,
            ));
        }
        if let Some(wings) = a.wings_total {
            filters.push(property(
                item,
                "Total Wings: #",
                "item.heist_wings_total",
                Some(wings as f64),
                0.,
                false,
            ));
        }
    }
    Preset { mode, filters }
}
fn area_props(item: &Item, range: f64, rarity_disabled: bool, filters: &mut Vec<Candidate>) {
    for (reference, id, value, disabled) in [
        (
            "Item Quantity: +#%",
            "item.map_item_quantity",
            item.area.quantity,
            false,
        ),
        (
            "Item Rarity: +#%",
            "item.map_item_rarity",
            item.area.rarity,
            rarity_disabled,
        ),
        (
            "Monster Pack Size: +#%",
            "item.map_pack_size",
            item.area.pack_size,
            false,
        ),
    ] {
        if let Some(value) = value.filter(|v| *v != 0.) {
            filters.push(property(item, reference, id, Some(value), range, disabled));
        }
    }
}

pub(super) fn flask_rules(item: &Item, filters: &mut Vec<Candidate>) {
    let enkindled = filters
        .iter()
        .any(|c| c.modifier.reference == "Gains no Charges during Flask Effect");
    for c in filters.iter_mut() {
        if c.modifier.kind == "enchant" && !enkindled {
            c.hidden = Some("hide_harvest_and_instilling".into());
            c.disabled = true;
        }
    }
    if item.rarity == Rarity::Magic
        && filters
            .iter()
            .any(|c| c.modifier.reference == "#% increased Charge Recovery")
        && !filters
            .iter()
            .any(|c| c.modifier.reference == "#% increased effect")
    {
        let index = filters
            .iter()
            .position(|c| c.modifier.reference == "#% increased Charge Recovery")
            .unwrap()
            + 1;
        filters.insert(
            index,
            exclusion(item, "#% increased effect", "explicit", true),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::{self, Filters, StatFilter};
    fn query(item: &Item, mode: Mode) -> serde_json::Value {
        let preset = create(item, 10., mode);
        filters::query(
            item,
            &Filters {
                stats: preset
                    .filters
                    .iter()
                    .filter(|c| !c.disabled)
                    .map(StatFilter::from_candidate)
                    .collect(),
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn map_properties_and_bulk_are_distinct_and_never_stat_ids() {
        let item = Item::parse(include_str!("fixtures/synthetic-map.txt")).unwrap();
        assert_eq!(available(&item), vec![Mode::Properties, Mode::Bulk]);
        let q = query(&item, Mode::Properties);
        assert_eq!(
            q["query"]["filters"]["map_filters"]["filters"]["map_iiq"]["min"],
            72.0
        );
        assert_eq!(
            q["query"]["filters"]["map_filters"]["filters"]["map_packsize"]["min"],
            22.0
        );
        assert_eq!(
            q["query"]["filters"]["map_filters"]["filters"]["map_blighted"]["option"],
            "false"
        );
        assert_eq!(q["query"]["stats"][0]["filters"], serde_json::json!([]));
        let bulk = query(&item, Mode::Bulk);
        assert!(bulk["query"]["filters"]["map_filters"]["filters"]
            .get("map_iiq")
            .is_none());
    }
    #[test]
    fn valued_map_drops_disable_rarity_and_eight_mods_keep_exact_count() {
        let mut item = Item::parse(include_str!("fixtures/synthetic-map.txt")).unwrap();
        item.area.more_scarabs = Some(40.);
        item.area.explicit_mods = 8;
        let p = create(&item, 10., Mode::Properties);
        assert!(
            p.filters
                .iter()
                .find(|c| c.modifier.id == "item.map_item_rarity")
                .unwrap()
                .disabled
        );
        let bulk = create(&item, 10., Mode::Bulk);
        assert_eq!(bulk.filters.len(), 1);
        assert_eq!(bulk.filters[0].initial_min, Some(8.));
        assert!(!bulk.filters[0].disabled);
    }
    #[test]
    fn absent_void_mod_is_excluded_only_for_reward_maps() {
        let mut item = Item::parse(include_str!("fixtures/synthetic-map.txt")).unwrap();
        assert!(create(&item, 10., Mode::Properties)
            .filters
            .iter()
            .all(|c| !c.exclude));
        item.area.completion_reward = Some("Kaom's Heart".into());
        let q = query(&item, Mode::Exact);
        assert_eq!(
            q["query"]["filters"]["map_filters"]["filters"]["map_completion_reward"]["option"],
            "Kaom's Heart"
        );
        assert_eq!(q["query"]["stats"][1]["type"], "not");
        assert_eq!(
            q["query"]["stats"][1]["filters"][0]["id"],
            "explicit.stat_1095765106"
        );
        let lethal = translation::translate(
            "Players who Die in area are sent to the Void",
            "explicit",
            Some("Map"),
            false,
        )
        .unwrap();
        item.modifiers.push(lethal);
        assert!(create(&item, 10., Mode::Exact)
            .filters
            .iter()
            .all(|c| !c.exclude));
    }
    #[test]
    fn heist_jobs_wings_and_missing_enchants_have_their_own_query_fields() {
        let item=Item::parse("Item Class: Contracts\nRarity: Rare\nSynthetic Fixture\nContract: Bunker\n--------\nArea Level: 83\nRequires Lockpicking (Level 3 (unmet))\nHeist Target: Fixture (Priceless)\n--------\nItem Level: 83").unwrap();
        let q = query(&item, Mode::Exact);
        assert!(q["query"].get("type").is_none());
        assert_eq!(
            q["query"]["filters"]["type_filters"]["filters"]["category"]["option"],
            "heistmission.contract"
        );
        assert_eq!(
            q["query"]["filters"]["heist_filters"]["filters"]["heist_lockpicking"]["min"],
            3.0
        );
        assert_eq!(
            q["query"]["filters"]["heist_filters"]["filters"]["heist_objective_value"]["option"],
            "priceless"
        );
        assert_eq!(
            q["query"]["filters"]["map_filters"]["filters"]["area_level"]["min"],
            83
        );
        let blueprint=Item::parse("Item Class: Blueprints\nRarity: Rare\nSynthetic Fixture\nBlueprint: Bunker\n--------\nArea Level: 83\nWings Revealed: 1/4\n--------\nItem Level: 83").unwrap();
        let q = query(&blueprint, Mode::Exact);
        assert_eq!(
            q["query"]["filters"]["heist_filters"]["filters"]["heist_wings"]["min"],
            1.0
        );
        assert_eq!(
            q["query"]["filters"]["heist_filters"]["filters"]["heist_max_wings"]["min"],
            4.0
        );
        assert_eq!(
            q["query"]["stats"][1]["filters"][0]["id"],
            "pseudo.pseudo_number_of_enchant_mods"
        );
    }
    #[test]
    fn magic_flask_uses_tier_and_avoids_the_unwanted_hybrid_prefix() {
        let text="Item Class: Utility Flasks\nRarity: Magic\nGranite Flask\n--------\nItem Level: 83\n--------\n{ Prefix Modifier \"Fixture\" (Tier: 1) }\n30(20-40)% increased Charge Recovery";
        let item = Item::parse(text).unwrap();
        let p = create(&item, 10., Mode::Exact);
        assert!(!p.filters[0].disabled);
        assert_eq!(p.filters[0].initial_min, Some(29.));
        assert!(p.filters[1].exclude);
        assert!(p.filters[1].modifier.text.contains("reduced effect"));
        let q = query(&item, Mode::Exact);
        assert_eq!(
            q["query"]["stats"][1]["filters"][0]["id"],
            "explicit.stat_2448920197"
        );
        let item = Item::parse(&text.replace("Tier: 1", "Tier: 3")).unwrap();
        assert!(create(&item, 10., Mode::Exact).filters[0].disabled);
    }
}
