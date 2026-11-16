//! Offline batch diagnostic for full item -> calculated filters comparisons.
#[allow(dead_code)]
#[path = "../src/calculated.rs"]
mod calculated;
#[allow(dead_code)]
#[path = "../src/item.rs"]
mod item;
#[allow(dead_code)]
#[path = "../src/modifiers.rs"]
mod modifiers;
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
  let equipment=properties::equipment(&item,i.range);
  let pseudos=pseudo::apply(&item,&equipment.remaining,i.range);
  Ok::<_,String>(serde_json::json!({"equipment":equipment.filters,"pseudo":pseudos.filters,"remaining":pseudos.remaining.iter().map(|m|serde_json::json!({"reference":m.reference,"kind":m.kind,"sources":m.source_count})).collect::<Vec<_>>() }))
 }).collect::<Result<_,_>>()?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
