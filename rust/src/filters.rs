use crate::item::{Item, Rarity};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filters {
    pub online: bool,
    pub min_item_level: Option<u32>,
    pub min_links: Option<u32>,
    pub stats: Vec<StatFilter>,
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
        }
    }
}
pub fn query(item: &Item, filters: &Filters) -> Value {
    let mut q = json!({"status":{"option":if filters.online {"online"} else {"any"}},"type":item.base,"stats":[{"type":"and","filters":filters.stats.iter().map(|s| json!({"id":s.id,"value":{"min":s.min,"max":s.max}})).collect::<Vec<_>>() }],"filters":{}});
    if item.rarity == Rarity::Unique && !item.unidentified {
        q["name"] = json!(item.name);
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
#[cfg(test)]
mod tests {
    use super::*;
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
