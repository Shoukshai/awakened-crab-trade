//! Offline diagnostic for full equipment presets and Trade query comparisons.
#[allow(dead_code)]
#[path = "../src/calculated.rs"]
mod calculated;
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
use serde::Deserialize;
#[derive(Deserialize)]
struct Input {
    text: String,
    range: f64,
    #[serde(default)]
    enable_empty: bool,
    #[serde(default)]
    enable_all: bool,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024)
        .read_to_string(&mut input)?;
    let inputs: Vec<Input> = serde_json::from_str(&input)?;
    let output:Vec<_>=inputs.into_iter().map(|i|{
  let item=item::Item::parse(&i.text)?;
  let modes=presets::available(&item);
  let cases:Vec<_>=modes.iter().map(|m|{
   let mut preset=presets::create(&item,i.range,*m);
   if i.enable_all { for f in &mut preset.filters {f.disabled=false;} }
   if i.enable_empty {for f in &mut preset.filters {if f.modifier.id=="item.has_empty_modifier" {f.disabled=false;}}}
   let plan=item_filters::create(&item,&item_filters::Options {league:"Covens".into(),merchant_only:false,exact:m.exact(),..Default::default()}).unwrap();
   let query=filters::query(&item,&filters::Filters {item:Some(plan.clone()),stats:preset.filters.iter().filter(|c|!c.disabled).map(filters::StatFilter::from_candidate).collect(),..Default::default()}).unwrap();
   serde_json::json!({"mode":m.id(),"filters":preset.filters,"plan":plan,"query":query})
  }).collect();
  Ok::<_,String>(serde_json::json!({"modes":modes.iter().map(|m|m.id()).collect::<Vec<_>>(),"presets":cases,"area":item.area,"modifiers":item.modifiers,"unknown":item.unknown_modifiers}))
 }).collect::<Result<_,_>>()?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
