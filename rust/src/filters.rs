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
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StatFilter {
    pub id: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub alternatives: Vec<String>,
    pub option: Option<f64>,
    pub roll_value: Option<f64>,
    pub inverted: bool,
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
        }
    }
}
pub fn query(item: &Item, filters: &Filters) -> Value {
    let mut and = Vec::new();
    let mut groups = Vec::new();
    for stat in &filters.stats {
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
    let mut q = json!({"status":{"option":if filters.online {"online"} else {"any"}},"type":item.base,"stats":groups,"filters":{}});
    if item.rarity == Rarity::Unique && !item.unidentified {
        q["name"] = json!(item.name);
    }
    if let Some(discriminator) = &item.trade_discriminator {
        q["type"] = json!({"option":item.base,"discriminator":discriminator});
    }
    if matches!(item.rarity, Rarity::Normal | Rarity::Magic | Rarity::Rare) {
        q["filters"]["type_filters"] = json!({"filters":{"rarity":{"option":"nonunique"}}});
    }
    let mut misc = json!({"corrupted":{"option":item.corrupted.to_string()}});
    if let Some(n) = filters.min_item_level {
        misc["ilvl"] = json!({"min":n});
    }
    if let Some(n) = item.gem_level {
        misc["gem_level"] = json!({"min":n});
    }
    if let Some(n) = item.quality.filter(|_| item.rarity == Rarity::Gem) {
        misc["quality"] = json!({"min":n});
    }
    q["filters"]["misc_filters"] = json!({"filters":misc});
    if let Some(n) = item.map_tier {
        q["filters"]["map_filters"] = json!({"filters":{"map_tier":{"min":n,"max":n}}});
    }
    if let Some(n) = filters
        .min_links
        .or_else(|| (item.links >= 5).then_some(item.links))
    {
        q["filters"]["socket_filters"] = json!({"filters":{"links":{"min":n}}});
    }
    json!({"query":q,"sort":{"price":"asc"}})
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
    let (id, fixed_option) = id
        .split_once('|')
        .map(|(id, value)| (id, value.parse::<f64>().ok()))
        .unwrap_or((id, None));
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
    if let Some(option) = fixed_option.or(stat.option) {
        value["option"] = json!(option);
    }
    json!({"id":id,"value":value})
}
/// Mirrors upstream's choice: stat filters require ordinary Trade even for currency.
pub fn search_query(item: &Item, filters: &Filters) -> Result<Value, String> {
    for stat in &filters.stats {
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
            if let Some(stock) = filters.minimum_stock {
                query["query"]["minimum"] = json!(stock);
            }
            return Ok(query);
        }
    }
    Ok(query(item, filters))
}
#[cfg(test)]
mod tests {
    use super::*;
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
        let body = query(&item, &filters);
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
            json!({"id":"enchant.stat_1","value":{"option":49.}})
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
    fn rare_query_uses_base_not_random_name() {
        let i = Item::parse(
            "Item Class: Rings\nRarity: Rare\nDoom Hold\nGold Ring\n--------\nItem Level: 83",
        )
        .unwrap();
        let q = query(&i, &Filters::default());
        assert_eq!(q["query"]["type"], "Gold Ring");
        assert!(q["query"].get("name").is_none());
    }
    #[test]
    fn unique_query_keeps_unique_identity() {
        let i =
            Item::parse("Item Class: Wands\nRarity: Unique\nLifesprig\nDriftwood Wand\n--------")
                .unwrap();
        assert_eq!(query(&i, &Filters::default())["query"]["name"], "Lifesprig");
    }
    #[test]
    fn map_tier_and_links_are_meaningful_filters() {
        let i = Item::parse("Item Class: Maps\nRarity: Normal\nBeach Map\n--------\nMap Tier: 16\nSockets: R-G-B-R-G-B").unwrap();
        let q = query(&i, &Filters::default());
        assert_eq!(
            q["query"]["filters"]["map_filters"]["filters"]["map_tier"]["min"],
            16
        );
        assert_eq!(
            q["query"]["filters"]["socket_filters"]["filters"]["links"]["min"],
            6
        );
    }
}
