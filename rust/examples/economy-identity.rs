//! Offline identity, feed, related prices, currency and trend comparison.
#[allow(dead_code)]
#[path = "../src/calculated.rs"]
mod calculated;
#[allow(dead_code)]
#[path = "../src/economy.rs"]
mod economy;
#[allow(dead_code)]
#[path = "../src/filters.rs"]
mod filters;
#[allow(dead_code)]
#[path = "../src/item.rs"]
mod item;
#[allow(dead_code)]
#[path = "../src/item_filters.rs"]
mod item_filters;
#[allow(dead_code)]
#[path = "../src/modifiers.rs"]
mod modifiers;
#[allow(dead_code)]
#[path = "../src/presets.rs"]
mod presets;
#[allow(dead_code)]
#[path = "../src/properties.rs"]
mod properties;
#[allow(dead_code)]
#[path = "../src/pseudo.rs"]
mod pseudo;
#[allow(dead_code)]
#[path = "../src/trade.rs"]
mod trade;
#[path = "../src/trade_item.rs"]
mod trade_item;
use serde::Deserialize;
#[derive(Deserialize)]
struct Money {
    value: f64,
    max: Option<f64>,
    rate: Option<f64>,
}
#[derive(Deserialize)]
struct Input {
    snapshot: serde_json::Value,
    league: String,
    items: Vec<String>,
    lookups: Vec<economy::Key>,
    relations: Vec<economy::Key>,
    slugs: Vec<economy::Key>,
    money: Vec<Money>,
    graphs: Vec<Vec<Option<f64>>>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut raw = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024)
        .read_to_string(&mut raw)?;
    let input: Input = serde_json::from_str(&raw)?;
    let cache: economy::Cached = serde_json::from_value(
        serde_json::json!({"version":1,"league":input.league,"fetched":100,"data":input.snapshot}),
    )?;
    let items: Vec<_> = input
        .items
        .iter()
        .map(|text| {
            let item = item::Item::parse(text)?;
            let key = economy::Key::for_item(&item);
            let report = key.as_ref().map(|key| cache.report(key, &item, 103, true));
            Ok::<_, String>(serde_json::json!({"key":key,"report":report}))
        })
        .collect::<Result<_, _>>()?;
    let rate = cache
        .find(
            &economy::Key {
                namespace: "ITEM".into(),
                name: "Divine Orb".into(),
                variant: None,
            },
            103,
            true,
        )
        .filter(|q| q.chaos >= 30.)
        .map(|q| q.chaos);
    println!(
        "{}",
        serde_json::json!({"items":items,"lookups":input.lookups.iter().map(|key|cache.find(key,103,true)).collect::<Vec<_>>(),"relations":input.relations.iter().map(|key|cache.related(key,103,true,rate)).collect::<Vec<_>>(),"slugs":input.slugs.iter().map(|key|key.slug()).collect::<Vec<_>>(),"money":input.money.iter().map(|m|economy::model::auto_currency(m.value,m.max,m.rate)).collect::<Vec<_>>(),"graphs":input.graphs.iter().map(|g|economy::model::Trend::of(g)).collect::<Vec<_>>()})
    );
    Ok(())
}
