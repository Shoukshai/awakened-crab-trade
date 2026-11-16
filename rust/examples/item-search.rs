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
#[path = "../src/inspection.rs"]
mod inspection;
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
use serde::Deserialize;
#[derive(Deserialize)]
struct Search {
    search: String,
    kind: item::search::Kind,
}
#[derive(Deserialize)]
struct Operation {
    search: String,
    chaos: Option<f64>,
}
#[derive(Deserialize)]
struct Input {
    cases: Vec<Search>,
    baskets: Vec<Vec<Operation>>,
}
fn preset_output(item: &item::Item) -> Result<Vec<serde_json::Value>, String> {
    presets::available(item)
        .into_iter()
        .map(|mode| {
            let preset = presets::create(item, 10., mode);
            let plan = item_filters::create(
                item,
                &item_filters::Options {
                    league: "Covens".into(),
                    merchant_only: false,
                    exact: mode.exact(),
                    ..Default::default()
                },
            )?;
            let query = filters::query(
                item,
                &filters::Filters {
                    item: Some(plan.clone()),
                    stats: preset
                        .filters
                        .iter()
                        .filter(|c| !c.disabled)
                        .map(filters::StatFilter::from_candidate)
                        .collect(),
                    ..Default::default()
                },
            )?;
            Ok(serde_json::json!({"mode":mode.id(),"plan":plan,"query":query}))
        })
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024)
        .read_to_string(&mut text)?;
    let input: Input = serde_json::from_str(&text)?;
    let cases=input.cases.into_iter().map(|c| {
  let choices=item::search::find(&c.search,c.kind);
  let virtuals=choices.as_ref().map(|choices|choices.iter().map(|c| {
   let item=c.virtual_item()?;
   let presets=preset_output(&item)?;
   let enriched=c.trade_item()?;
   let enriched_presets=preset_output(&enriched)?;
   Ok::<_,String>(serde_json::json!({"item":item,"presets":presets,"enriched_presets":enriched_presets,"quote_identity":economy::Key::for_item(&enriched)}))
  }).collect::<Result<Vec<_>,_>>()).transpose()?;
  Ok::<_,String>(serde_json::json!({"choices":choices,"virtuals":virtuals}))
 }).collect::<Result<Vec<_>,_>>()?;
    let baskets = input
        .baskets
        .into_iter()
        .map(|ops| {
            let mut basket = item::search::Basket::default();
            ops.into_iter()
                .map(|op| {
                    let choice = item::search::find(&op.search, item::search::Kind::Replica)
                        .and_then(|c| c.into_iter().next())
                        .ok_or("Unknown basket fixture")?;
                    let added = basket.add(choice, op.chaos);
                    Ok::<_, String>(serde_json::json!({"added":added,"items":basket.items}))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    println!("{}", serde_json::json!({"cases":cases,"baskets":baskets}));
    Ok(())
}
