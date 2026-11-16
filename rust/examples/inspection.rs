//! Offline batch inspection regression diagnostic.
#[allow(dead_code)]
#[path = "../src/calculated.rs"]
mod calculated;
#[allow(dead_code)]
#[path = "../src/inspection.rs"]
mod inspection;
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
    config: inspection::MapConfig,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024)
        .read_to_string(&mut input)?;
    let inputs: Vec<Input> = serde_json::from_str(&input)?;
    let output: Vec<_> = inputs
        .into_iter()
        .map(|i| {
            i.config.validate()?;
            let item = item::Item::parse(&i.text)?;
            Ok::<_, String>(inspection::report(&item, &i.config))
        })
        .collect::<Result<_, _>>()?;
    let mut decisions = Vec::new();
    for a in "-dwgs".chars() {
        for b in "-dwgs".chars() {
            for c in "-dwgs".chars() {
                let set = format!("{a}{b}{c}");
                for profile in 1..=3 {
                    for value in "-dwgs".chars() {
                        decisions.push(serde_json::json!({"set":set,"profile":profile,"value":value,"next":inspection::next_decision(value),"color":inspection::decision_has_color(&set,profile),"created":inspection::decision_create(value,profile,&set)}));
                    }
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"reports":output,"catalog":inspection::catalog(),"decisions":decisions})
    );
    Ok(())
}
