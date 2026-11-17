//! Displayed details of the offered item, taken from its Trade fetch response.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferedItem {
    pub name: String,
    pub base: String,
    pub item_level: Option<u64>,
    pub rarity: Option<u64>,
    #[serde(default)]
    pub gem_level: Option<u64>,
    #[serde(default)]
    pub quality: Option<u64>,
    #[serde(default)]
    pub map_tier: Option<u64>,
    pub properties: Vec<String>,
    pub requirements: Vec<String>,
    pub mods: Vec<ModGroup>,
    pub corrupted: bool,
    pub identified: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModGroup {
    pub kind: String,
    pub lines: Vec<String>,
}
fn properties(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            let name = p["name"].as_str().unwrap_or("");
            let values: Vec<_> = p["values"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v[0].as_str())
                .collect();
            if name.contains('%') {
                let mut text = name.to_string();
                for (i, v) in values.iter().enumerate().rev() {
                    text = text.replace(&format!("%{i}"), v);
                }
                text
            } else if name.is_empty() {
                values.join(", ")
            } else if values.is_empty() {
                name.into()
            } else {
                format!("{name}: {}", values.join(", "))
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}
impl OfferedItem {
    pub fn from_api(value: &Value) -> Option<Self> {
        if !value.is_object() {
            return None;
        }
        let name = value["name"].as_str().unwrap_or("").to_string();
        let base = value["typeLine"]
            .as_str()
            .or_else(|| value["baseType"].as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() && base.is_empty() {
            return None;
        }
        let mods = [
            "implicitMods",
            "explicitMods",
            "craftedMods",
            "fracturedMods",
            "enchantMods",
            "runeMods",
            "scourgeMods",
            "mutatedMods",
            "utilityMods",
            "veiledMods",
        ]
        .into_iter()
        .filter_map(|kind| {
            let lines: Vec<_> = value[kind]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            (!lines.is_empty()).then(|| ModGroup {
                kind: kind.into(),
                lines,
            })
        })
        .collect();
        let numeric = |name: &str| {
            value["properties"]
                .as_array()?
                .iter()
                .find(|p| p["name"].as_str() == Some(name))?["values"][0][0]
                .as_str()?
                .split(|c: char| !c.is_ascii_digit())
                .find(|s| !s.is_empty())?
                .parse::<u64>()
                .ok()
        };
        Some(Self {
            gem_level: numeric("Level"),
            quality: numeric("Quality"),
            map_tier: numeric("Map Tier").or_else(|| {
                base.rsplit_once(" (Tier ")
                    .and_then(|(_, v)| v.strip_suffix(')'))
                    .and_then(|v| v.parse().ok())
            }),
            name,
            base,
            item_level: value["ilvl"].as_u64(),
            rarity: value["frameType"].as_u64(),
            properties: properties(&value["properties"]),
            requirements: properties(&value["requirements"]),
            mods,
            corrupted: value["corrupted"].as_bool().unwrap_or(false),
            identified: value["identified"].as_bool(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offered_values_are_preserved_separately_from_the_searched_item() {
        let item=OfferedItem::from_api(&serde_json::json!({"name":"Fixture seller's ring","typeLine":"Gold Ring","ilvl":84,"frameType":2,"properties":[{"name":"Quality","values":[["+20%",1]]}],"requirements":[{"name":"Level","values":[["64",0]]}],"implicitMods":["12% increased Rarity of Items found"],"explicitMods":["+110 to maximum Life","+42% to Fire Resistance"],"craftedMods":["+16% to Chaos Resistance"],"corrupted":true})).unwrap();
        assert_eq!(item.properties, ["Quality: +20%"]);
        assert_eq!(item.requirements, ["Level: 64"]);
        assert_eq!(item.item_level, Some(84));
        assert!(item.corrupted);
        assert_eq!(
            item.mods[1].lines,
            ["+110 to maximum Life", "+42% to Fire Resistance"]
        );
        assert_eq!(item.mods[2].kind, "craftedMods");
    }
    #[test]
    fn templates_and_unavailable_item_data_are_handled_without_guessed_stats() {
        assert!(OfferedItem::from_api(&Value::Null).is_none());
        assert!(OfferedItem::from_api(&serde_json::json!({})).is_none());
        let p =
            properties(&serde_json::json!([{"name":"%0 to %1 Damage","values":[["5",0],["9",0]]}]));
        assert_eq!(p, ["5 to 9 Damage"]);
        let item = OfferedItem::from_api(
            &serde_json::json!({"typeLine":"Driftwood Wand","identified":false}),
        )
        .unwrap();
        assert!(item.mods.is_empty());
        assert_eq!(item.identified, Some(false));
    }
}
