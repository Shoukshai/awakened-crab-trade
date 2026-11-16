//! Offline diagnostic for specialised preset and Trade query comparisons.
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
   let preset=presets::create(&item,i.range,*m);
   let query=filters::query(&item,&filters::Filters {stats:preset.filters.iter().filter(|c|!c.disabled).map(filters::StatFilter::from_candidate).collect(),..Default::default()});
   serde_json::json!({"mode":m.id(),"filters":preset.filters,"query":query})
  }).collect();
  Ok::<_,String>(serde_json::json!({"modes":modes.iter().map(|m|m.id()).collect::<Vec<_>>(),"presets":cases,"area":item.area,"modifiers":item.modifiers}))
 }).collect::<Result<_,_>>()?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
