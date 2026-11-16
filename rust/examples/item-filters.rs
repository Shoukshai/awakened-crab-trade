//! Full parser -> item policy -> query diagnostic, with no desktop/network calls.
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
    options: item_filters::Options,
    overrides: Option<serde_json::Value>,
}
fn merge(to: &mut serde_json::Value, from: &serde_json::Value) {
    if let Some(patch) = from.as_object() {
        if !to.is_object() {
            *to = serde_json::json!({});
        }
        for (key, value) in patch {
            merge(
                to.as_object_mut()
                    .unwrap()
                    .entry(key.clone())
                    .or_insert(serde_json::Value::Null),
                value,
            );
        }
    } else {
        *to = from.clone();
    }
}
fn run(input: Input) -> Result<serde_json::Value, String> {
    let item = item::Item::parse(&input.text)?;
    let mut plan = item_filters::create(&item, &input.options)?;
    if let Some(edits) = input.overrides {
        let mut value = serde_json::to_value(&plan).map_err(|e| e.to_string())?;
        merge(&mut value, &edits);
        plan = serde_json::from_value(value).map_err(|e| e.to_string())?;
    }
    let query = filters::query(
        &item,
        &filters::Filters {
            item: Some(plan.clone()),
            ..Default::default()
        },
    )?;
    Ok(
        serde_json::json!({"plan":plan,"query":query,"metadata":item.market,"category":item.category}),
    )
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
        .map(|input| match run(input) {
            Ok(value) => value,
            Err(error) => serde_json::json!({"error":error}),
        })
        .collect();
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
