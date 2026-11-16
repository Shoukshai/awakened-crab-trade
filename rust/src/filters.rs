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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatFilter {
    pub id: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
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
    let mut q = json!({"status":{"option":if filters.online {"online"} else {"any"}},"type":item.base,"stats":[{"type":"and","filters":filters.stats.iter().map(|s| json!({"id":s.id,"value":{"min":s.min,"max":s.max}})).collect::<Vec<_>>() }],"filters":{}});
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
/// Mirrors upstream's choice: stat filters require ordinary Trade even for currency.
pub fn search_query(item: &Item, filters: &Filters) -> Result<Value, String> {
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
