use crate::item::{Item, Rarity};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
pub const PAYMENT_CURRENCIES: &[(&str, &str)] = &[
    ("chaos", "Chaos Orb"),
    ("divine", "Divine Orb"),
    ("exalted", "Exalted Orb"),
    ("alch", "Orb of Alchemy"),
    ("alt", "Orb of Alteration"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Filters {
    pub online: bool,
    pub min_item_level: Option<u32>,
    pub min_links: Option<u32>,
    pub stats: Vec<StatFilter>,
    pub bulk_currency: String,
    pub minimum_stock: Option<u32>,
    pub merchant_only: bool,
    pub collapse_listings: bool,
    #[serde(skip)]
    pub item: Option<crate::item_filters::Plan>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StatFilter {
    pub group: Option<String>,
    #[serde(rename = "not")]
    pub exclude: bool,
    pub id: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub alternatives: Vec<String>,
    pub option: Option<f64>,
    pub roll_value: Option<f64>,
    pub inverted: bool,
}
impl StatFilter {
    pub fn from_candidate(candidate: &crate::calculated::Candidate) -> Self {
        let m = &candidate.modifier;
        let numeric = !m.option && !m.ids.iter().any(|id| id.contains('|')) && m.value.is_some();
        Self {
            group: candidate.group.clone(),
            exclude: candidate.exclude,
            id: m.id.clone(),
            alternatives: m.ids.iter().skip(1).cloned().collect(),
            min: numeric.then_some(candidate.initial_min).flatten(),
            max: numeric.then_some(candidate.initial_max).flatten(),
            option: m.option.then_some(m.value).flatten(),
            roll_value: candidate.display_value,
            inverted: m.inverted,
        }
    }
}
impl Default for Filters {
    fn default() -> Self {
        Self {
            online: true,
            min_item_level: None,
            min_links: None,
            stats: vec![],
            bulk_currency: "chaos".into(),
            minimum_stock: None,
            merchant_only: true,
            collapse_listings: true,
            item: None,
        }
    }
}
pub fn plan(item: &Item, filters: &Filters) -> Result<crate::item_filters::Plan, String> {
    plan_for_league(item, filters, "Standard")
}
pub fn plan_for_league(
    item: &Item,
    filters: &Filters,
    league: &str,
) -> Result<crate::item_filters::Plan, String> {
    plan_for_preset(
        item,
        filters,
        league,
        crate::item_filters::exact_default(item),
    )
}
pub fn plan_for_preset(
    item: &Item,
    filters: &Filters,
    league: &str,
    exact: bool,
) -> Result<crate::item_filters::Plan, String> {
    if let Some(plan) = &filters.item {
        return Ok(plan.clone());
    }
    let mut p = crate::item_filters::create(
        item,
        &crate::item_filters::Options {
            league: league.into(),
            exact,
            merchant_only: filters.merchant_only,
            collapse_listings: if filters.collapse_listings {
                "api".into()
            } else {
                "app".into()
            },
            ..Default::default()
        },
    )?;
    p.trade.offline = !filters.online;
    if let Some(n) = filters.min_item_level {
        p.item_level = Some(crate::item_filters::Numeric::new(n, false));
    }
    if let Some(n) = filters.min_links {
        p.linked_sockets = Some(crate::item_filters::Numeric::new(n, false));
    }
    if let Some(n) = filters.minimum_stock {
        p.stack_size = Some(crate::item_filters::Numeric::new(n, false));
    }
    Ok(p)
}
pub fn query(item: &Item, filters: &Filters) -> Result<Value, String> {
    let plan = plan(item, filters)?;
    let mut and = Vec::new();
    let mut groups = Vec::new();
    let mut properties = Vec::new();
    let mut exclusions = Vec::new();
    let mut one_groups: Vec<(String, Vec<Value>)> = Vec::new();
    for stat in &filters.stats {
        if let Some(name) = &stat.group {
            let values = std::iter::once(&stat.id)
                .chain(stat.alternatives.iter())
                .map(|id| stat_to_query(id, stat));
            if let Some((_, group)) = one_groups.iter_mut().find(|(key, _)| key == name) {
                group.extend(values);
            } else {
                one_groups.push((name.clone(), values.collect()));
            }
            continue;
        }
        if stat.id == "item.has_empty_modifier" {
            let option = stat.option.unwrap_or(0.) as usize;
            if option > 2 {
                return Err("Type d’emplacement libre inconnu.".into());
            }
            let empty = crate::modifiers::translation::pseudo(
                [
                    "# Empty Modifiers",
                    "# Empty Prefix Modifiers",
                    "# Empty Suffix Modifiers",
                ][option],
            )
            .ok_or("Filtre d’emplacement libre absent.")?
            .id;
            let crafted = crate::modifiers::translation::pseudo(
                [
                    "# Crafted Modifiers",
                    "# Crafted Prefix Modifiers",
                    "# Crafted Suffix Modifiers",
                ][option],
            )
            .ok_or("Filtre craft absent.")?
            .id;
            let total = crate::modifiers::translation::pseudo("# Modifiers")
                .ok_or("Filtre de mods absent.")?
                .id;
            for (id, min) in [(crafted, 1), (total, 6)] {
                groups.push(json!({"type":"count","value":{"min":1,"max":1},"disabled":false,"filters":[{"id":empty,"value":{"min":1,"max":1},"disabled":false},{"id":id,"value":{"min":min},"disabled":false}]}));
            }
            continue;
        }
        if stat.exclude {
            exclusions.extend(
                std::iter::once(&stat.id)
                    .chain(stat.alternatives.iter())
                    .map(|id| stat_to_query(id, stat)),
            );
            continue;
        }
        if let Some((group, property)) = property_path(&stat.id) {
            properties.push((group, property, stat));
            continue;
        }
        let ids: Vec<_> = std::iter::once(&stat.id)
            .chain(stat.alternatives.iter())
            .collect();
        let values: Vec<_> = ids.iter().map(|id| stat_to_query(id, stat)).collect();
        if values.len() == 1 {
            and.extend(values);
        } else {
            groups.push(json!({"type":"count","value":{"min":1},"filters":values}));
        }
    }
    groups.insert(0, json!({"type":"and","filters":and}));
    for (_, filters) in one_groups {
        groups.push(json!({"type":"count","value":{"min":1},"disabled":false,"filters":filters}));
    }
    if !exclusions.is_empty() {
        groups.push(json!({"type":"not","filters":exclusions}));
    }
    let mut q = json!({"stats":groups,"filters":{}});
    plan.append_query(&mut q)?;
    for (group, property, stat) in properties {
        let mut value = stat_to_query(&stat.id, stat)["value"].clone();
        if stat.id == "item.heist_target_priceless" {
            value = json!({"option":"priceless"});
        } else if stat.id.starts_with("item.heist_job_") && stat.min.is_none() {
            value["min"] = json!(1);
        }
        q["filters"][group]["filters"][property] = value;
    }
    Ok(json!({"query":q,"sort":{"price":"asc"}}))
}
fn property_path(id: &str) -> Option<(&'static str, &'static str)> {
    Some(match id {
        "item.armour" => ("armour_filters", "ar"),
        "item.evasion_rating" => ("armour_filters", "ev"),
        "item.energy_shield" => ("armour_filters", "es"),
        "item.ward" => ("armour_filters", "ward"),
        "item.block" => ("armour_filters", "block"),
        "item.base_percentile" => ("armour_filters", "base_defence_percentile"),
        "item.total_dps" => ("weapon_filters", "dps"),
        "item.physical_dps" => ("weapon_filters", "pdps"),
        "item.elemental_dps" => ("weapon_filters", "edps"),
        "item.aps" => ("weapon_filters", "aps"),
        "item.crit" => ("weapon_filters", "crit"),
        "item.memory_strands" => ("misc_filters", "memory_level"),
        "item.map_item_quantity" => ("map_filters", "map_iiq"),
        "item.map_item_rarity" => ("map_filters", "map_iir"),
        "item.map_pack_size" => ("map_filters", "map_packsize"),
        "item.chart_sulphur" => ("map_filters", "chart_sulphur"),
        "item.heist_job_lockpicking" => ("heist_filters", "heist_lockpicking"),
        "item.heist_job_bruteforce" => ("heist_filters", "heist_brute_force"),
        "item.heist_job_perception" => ("heist_filters", "heist_perception"),
        "item.heist_job_demolition" => ("heist_filters", "heist_demolition"),
        "item.heist_job_counterthaumaturgy" => ("heist_filters", "heist_counter_thaumaturgy"),
        "item.heist_job_trapdisarmament" => ("heist_filters", "heist_trap_disarmament"),
        "item.heist_job_agility" => ("heist_filters", "heist_agility"),
        "item.heist_job_deception" => ("heist_filters", "heist_deception"),
        "item.heist_job_engineering" => ("heist_filters", "heist_engineering"),
        "item.heist_target_priceless" => ("heist_filters", "heist_objective_value"),
        "item.heist_wings_revealed" => ("heist_filters", "heist_wings"),
        "item.heist_wings_total" => ("heist_filters", "heist_max_wings"),
        _ => return None,
    })
}
fn stat_to_query(id: &str, stat: &StatFilter) -> Value {
    let divisor = if id.starts_with("{div_by_100}") {
        100.
    } else {
        1.
    };
    let empty = id.starts_with("{empty}")
        || (id.starts_with("{empty_if_100}") && stat.roll_value == Some(100.));
    let id = if id.starts_with('{') {
        id.split_once('}').map(|(_, s)| s).unwrap_or(id)
    } else {
        id
    };
    let mut value = json!({});
    if !empty {
        let (min, max) = if stat.inverted {
            (stat.max.map(|v| -v), stat.min.map(|v| -v))
        } else {
            (stat.min, stat.max)
        };
        if let Some(min) = min {
            value["min"] = json!(min / divisor);
        }
        if let Some(max) = max {
            value["max"] = json!(max / divisor);
        }
    }
    if let Some(option) = stat.option {
        value["option"] = json!(option);
    }
    json!({"id":id,"value":value})
}
/// Mirrors upstream's choice: stat filters require ordinary Trade even for currency.
pub fn search_query(item: &Item, filters: &Filters) -> Result<Value, String> {
    if item.reference == "Expedition Logbook" && item.logbook_areas.is_empty() {
        return Err("Zones du logbook absentes ou non reconnues. Copiez sa description avancée avant de rechercher.".into());
    }
    if item.rarity != Rarity::Unique
        && [
            Some("Heist Contract"),
            Some("Heist Blueprint"),
            Some("Chart"),
        ]
        .contains(&item.category.as_deref())
        && item.area.level.is_none()
    {
        return Err("Le niveau de zone manque dans la copie ; recherche refusée.".into());
    }
    if item.rarity != Rarity::Unique
        && item.category.as_deref() == Some("Heist Blueprint")
        && (item.area.wings_revealed.is_none() || item.area.wings_total.is_none())
    {
        return Err("Le nombre d’ailes manque dans la copie ; recherche refusée.".into());
    }
    if (item.category.as_deref() == Some("Chart")
        || matches!(
            item.reference.as_str(),
            "Blighted Map" | "Blight-ravaged Map"
        ))
        && item.area.trade_disc.is_none()
    {
        return Err("La zone de cet objet n’est pas reconnue ; recherche refusée.".into());
    }
    for stat in &filters.stats {
        if stat.id.starts_with("item.")
            && stat.id != "item.has_empty_modifier"
            && property_path(&stat.id).is_none()
        {
            return Err("Cette propriété n’est pas encore prise en charge.".into());
        }
        if [stat.min, stat.max, stat.option, stat.roll_value]
            .into_iter()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err("Les valeurs des mods doivent être des nombres finis.".into());
        }
        if stat.min.zip(stat.max).is_some_and(|(min, max)| min > max) {
            return Err("Le minimum d’un mod dépasse son maximum.".into());
        }
    }
    if filters.stats.is_empty() {
        if let Some(want) = &item.trade_tag {
            let have = &filters.bulk_currency;
            if !PAYMENT_CURRENCIES.iter().any(|(tag, _)| *tag == have) {
                return Err("Unsupported exchange currency".into());
            }
            if want == have {
                return Err("Choose a payment currency different from this item".into());
            }
            let mut query = json!({"engine":"new","query":{"status":{"option":if filters.online {"online"} else {"any"}},"have":[have],"want":[want]},"sort":{"have":"asc"}});
            let stock = if let Some(p) = &filters.item {
                p.stack_size
                    .as_ref()
                    .filter(|f| !f.disabled)
                    .map(|f| f.value)
            } else {
                filters.minimum_stock
            };
            if let Some(stock) = stock {
                query["query"]["minimum"] = json!(stock);
            }
            return Ok(query);
        }
    }
    query(item, filters)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_league_changes_the_split_policy_and_keeps_legacy_overrides() {
        let item = Item::parse(include_str!("fixtures/synthetic-calculated-ring.txt")).unwrap();
        let filters = Filters {
            online: false,
            min_item_level: Some(80),
            ..Default::default()
        };
        let standard = plan_for_league(&item, &filters, "Standard").unwrap();
        let league = plan_for_league(&item, &filters, "Covens").unwrap();
        assert!(standard.split.is_none());
        assert!(league.split.as_ref().unwrap().disabled);
        assert!(league.trade.offline);
        assert_eq!(league.trade.league, "Covens");
        assert_eq!(league.item_level.unwrap().value, 80);
    }
    #[test]
    fn calculated_weapon_property_is_routed_to_weapon_filters_and_keeps_other_constraints() {
        let i = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        let c = crate::calculated::candidates(&i, 10.)
            .into_iter()
            .find(|c| c.modifier.id == "item.physical_dps")
            .unwrap();
        let filters = Filters {
            min_item_level: Some(1),
            stats: vec![StatFilter::from_candidate(&c)],
            ..Filters::default()
        };
        let q = search_query(&i, &filters).unwrap();
        assert_eq!(
            q["query"]["filters"]["weapon_filters"]["filters"]["pdps"]["min"],
            12.
        );
        assert_eq!(
            q["query"]["filters"]["misc_filters"]["filters"]["ilvl"]["min"],
            1
        );
        assert_eq!(q["query"]["stats"][0]["filters"], json!([]));
    }
    #[test]
    fn negative_pseudo_display_maps_back_to_correct_trade_bound() {
        let i=Item::parse("Item Class: Rings\nRarity: Rare\nSynthetic Fixture\nGold Ring\n--------\n{ Suffix Modifier }\n20(15-30)% reduced Movement Speed").unwrap();
        let c = crate::calculated::candidates(&i, 10.)
            .into_iter()
            .find(|c| c.modifier.kind == "pseudo")
            .unwrap();
        assert_eq!(c.initial_min, None);
        assert_eq!(c.initial_max, Some(22.));
        let filters = Filters {
            stats: vec![StatFilter::from_candidate(&c)],
            ..Filters::default()
        };
        let q = search_query(&i, &filters).unwrap();
        assert_eq!(
            q["query"]["stats"][0]["filters"][0]["value"],
            json!({"min":-22.})
        );
    }
    #[test]
    fn alternative_ids_form_one_or_condition_instead_of_requiring_both() {
        let item =
            Item::parse("Item Class: Utility Flasks\nRarity: Magic\nGranite Flask\n--------")
                .unwrap();
        let filters = Filters {
            stats: vec![StatFilter {
                id: "explicit.stat_1619168299".into(),
                alternatives: vec!["explicit.stat_1053326368".into()],
                min: Some(7.),
                ..StatFilter::default()
            }],
            ..Filters::default()
        };
        let body = query(&item, &filters).unwrap();
        assert_eq!(body["query"]["stats"][0]["filters"], json!([]));
        let group = &body["query"]["stats"][1];
        assert_eq!(group["type"], "count");
        assert_eq!(group["value"]["min"], 1);
        assert_eq!(group["filters"].as_array().unwrap().len(), 2);
        assert_eq!(group["filters"][1]["value"]["min"], 7.);
    }
    #[test]
    fn trade_transforms_options_and_inverse_bounds_match_upstream() {
        let stat = StatFilter {
            min: Some(20.),
            max: Some(80.),
            roll_value: Some(100.),
            ..StatFilter::default()
        };
        assert_eq!(
            stat_to_query("{div_by_100}explicit.stat_1", &stat),
            json!({"id":"explicit.stat_1","value":{"min":0.2,"max":0.8}})
        );
        assert_eq!(
            stat_to_query("{empty_if_100}explicit.stat_1", &stat),
            json!({"id":"explicit.stat_1","value":{}})
        );
        assert_eq!(
            stat_to_query("{empty}explicit.stat_1", &stat)["value"],
            json!({})
        );
        assert_eq!(
            stat_to_query("enchant.stat_1|49", &StatFilter::default()),
            json!({"id":"enchant.stat_1|49","value":{}})
        );
        let inverted = StatFilter {
            inverted: true,
            ..stat
        };
        assert_eq!(
            stat_to_query("explicit.stat_1", &inverted)["value"],
            json!({"min":-80.,"max":-20.})
        );
        let old: StatFilter =
            serde_json::from_str(r#"{"id":"explicit.stat_1","min":8,"max":null}"#).unwrap();
        assert!(old.alternatives.is_empty());
        assert_eq!(stat_to_query(&old.id, &old)["value"], json!({"min":8.}));
    }
    #[test]
    fn invalid_stat_ranges_are_rejected_before_a_search_is_queued() {
        let item = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        let mut filters = Filters {
            stats: vec![StatFilter {
                id: "implicit.stat_2974417149".into(),
                min: Some(12.),
                max: Some(8.),
                ..StatFilter::default()
            }],
            ..Filters::default()
        };
        assert!(search_query(&item, &filters)
            .unwrap_err()
            .contains("minimum"));
        filters.stats[0].max = Some(f64::NAN);
        assert!(search_query(&item, &filters).unwrap_err().contains("finis"));
    }
    #[test]
    fn currency_exchange_uses_catalogue_tags_and_stock() {
        let item=Item::parse("Item Class: Stackable Currency\nRarity: Currency\nScroll of Wisdom\n--------\nStack Size: 9/40").unwrap();
        assert_eq!(item.trade_tag.as_deref(), Some("wisdom"));
        assert_eq!(item.stack_size, Some(9));
        let filters = Filters {
            minimum_stock: Some(100),
            ..Filters::default()
        };
        let body = search_query(&item, &filters).unwrap();
        assert_eq!(body["engine"], "new");
        assert_eq!(body["query"]["have"][0], "chaos");
        assert_eq!(body["query"]["want"][0], "wisdom");
        assert_eq!(body["query"]["minimum"], 100);
    }
    #[test]
    fn selected_stats_force_item_search_and_same_currency_is_rejected() {
        let item =
            Item::parse("Item Class: Stackable Currency\nRarity: Currency\nChaos Orb\n--------")
                .unwrap();
        assert!(search_query(&item, &Filters::default()).is_err());
        let filters = Filters {
            stats: vec![StatFilter {
                id: "explicit.stat_test".into(),
                min: Some(1.),
                max: None,
                ..StatFilter::default()
            }],
            ..Filters::default()
        };
        assert!(search_query(&item, &filters)
            .unwrap()
            .get("engine")
            .is_none());
    }
    #[test]
    fn exchangeable_without_a_tag_does_not_invent_an_exchange_id() {
        let item = Item::parse(
            "Item Class: Divination Cards\nRarity: Divination Card\nThe Doctor\n--------",
        )
        .unwrap();
        assert_eq!(item.namespace, "DIVINATION_CARD");
        assert!(item.trade_tag.is_none());
        assert!(search_query(&item, &Filters::default())
            .unwrap()
            .get("engine")
            .is_none());
    }
    #[test]
    fn rare_query_uses_category_or_explicit_base_without_random_name() {
        let i = Item::parse(
            "Item Class: Rings\nRarity: Rare\nDoom Circle\nGold Ring\n--------\nItem Level: 83",
        )
        .unwrap();
        let q = query(&i, &Filters::default()).unwrap();
        assert!(q["query"].get("type").is_none());
        assert!(q["query"].get("name").is_none());
        assert_eq!(
            q["query"]["filters"]["type_filters"]["filters"]["category"]["option"],
            "accessory.ring"
        );
        let plan = crate::item_filters::create(
            &i,
            &crate::item_filters::Options {
                exact: true,
                ..Default::default()
            },
        )
        .unwrap();
        let exact = query(
            &i,
            &Filters {
                item: Some(plan),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(exact["query"]["type"], "Gold Ring");
        assert!(exact["query"].get("name").is_none());
    }
    #[test]
    fn unique_query_keeps_unique_identity() {
        let i =
            Item::parse("Item Class: Wands\nRarity: Unique\nLifesprig\nDriftwood Wand\n--------")
                .unwrap();
        assert_eq!(
            query(&i, &Filters::default()).unwrap()["query"]["name"],
            "Lifesprig"
        );
    }
    #[test]
    fn map_tier_and_equipment_links_use_their_own_filters() {
        let map = Item::parse(include_str!("fixtures/synthetic-map.txt")).unwrap();
        let q = query(&map, &Filters::default()).unwrap();
        assert_eq!(
            q["query"]["filters"]["map_filters"]["filters"]["map_tier"],
            json!({"min":16,"max":16})
        );
        let armour=Item::parse("Item Class: Body Armours\nRarity: Rare\nDoom Vest\nPlate Vest\n--------\nArmour: 16\n--------\nSockets: R-R-R-R-R-R\n--------\nItem Level: 83").unwrap();
        let q = query(&armour, &Filters::default()).unwrap();
        assert_eq!(
            q["query"]["filters"]["socket_filters"]["filters"]["links"],
            json!({"min":6})
        );
    }
}

#[cfg(test)]
mod exclusion_tests {
    use super::*;
    #[test]
    fn all_alternative_exclusion_ids_are_in_one_not_group() {
        let item = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        let f = StatFilter {
            id: "explicit.one".into(),
            alternatives: vec!["explicit.two".into()],
            exclude: true,
            ..Default::default()
        };
        let q = query(
            &item,
            &Filters {
                stats: vec![f],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(q["query"]["stats"][0]["filters"], json!([]));
        assert_eq!(q["query"]["stats"][1]["type"], "not");
        assert_eq!(
            q["query"]["stats"][1]["filters"].as_array().unwrap().len(),
            2
        );
    }
}

#[cfg(test)]
mod incomplete_area_tests {
    use super::*;
    #[test]
    fn incomplete_blueprint_or_unknown_chart_area_never_broadens_a_search() {
        let incomplete="Item Class: Blueprints\nRarity: Rare\nFixture\nBlueprint: Bunker\n--------\nItem Level: 83";
        let item = Item::parse(incomplete).unwrap();
        assert!(search_query(&item, &Filters::default())
            .unwrap_err()
            .contains("niveau de zone"));
        let item = Item::parse(&format!("{incomplete}\nArea Level: 83")).unwrap();
        assert!(search_query(&item, &Filters::default())
            .unwrap_err()
            .contains("ailes"));
        let mut item = item;
        item.category = Some("Chart".into());
        assert!(search_query(&item, &Filters::default())
            .unwrap_err()
            .contains("zone"));
    }
}
