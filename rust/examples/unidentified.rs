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
#[path = "../src/trade_item.rs"]
mod trade_item;
use serde::Deserialize;
#[derive(Deserialize)]
struct Input {
    text: String,
    select: Option<String>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024)
        .read_to_string(&mut text)?;
    let inputs: Vec<Input> = serde_json::from_str(&text)?;
    let outputs = inputs.into_iter().map(|input| {
  let original = item::Item::parse(&input.text)?;
  let choices = original.unique_identity_choices();
  let selected = input.select.as_ref().map(|r| original.with_unique_identity(r));
  let result = selected.map(|result| result.and_then(|item| {
   let presets = presets::available(&item).into_iter().map(|mode| {
    let preset = presets::create(&item,10.,mode);
    let plan = item_filters::create(&item,&item_filters::Options{league:"Covens".into(),merchant_only:false,exact:mode.exact(),..Default::default()})?;
    let query = filters::query(&item,&filters::Filters{item:Some(plan.clone()),stats:preset.filters.iter().filter(|c| !c.disabled).map(filters::StatFilter::from_candidate).collect(),..Default::default()})?;
    Ok::<_,String>(serde_json::json!({"mode":mode.id(),"plan":plan,"query":query}))
   }).collect::<Result<Vec<_>,_>>()?;
   Ok::<_,String>(serde_json::json!({"item":item,"dust":inspection::dust(&item),"key":economy::Key::for_item(&item),"presets":presets}))
  }));
  Ok::<_,String>(serde_json::json!({"needs_identity":original.needs_unique_identity(),"choices":choices,"selected":result}))
 }).collect::<Result<Vec<_>,_>>()?;
    println!("{}", serde_json::to_string(&outputs)?);
    Ok(())
}
