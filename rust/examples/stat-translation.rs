//! Offline batch diagnostic for comparison with the unchanged upstream parser.
#[allow(dead_code)]
#[path = "../src/modifiers.rs"]
mod modifiers;
use serde::Deserialize;
#[derive(Deserialize)]
struct Input {
    text: String,
    kind: String,
    category: Option<String>,
    #[serde(default)]
    unscalable: bool,
    #[serde(default)]
    effect: f64,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .take(32 * 1024 * 1024)
        .read_to_string(&mut input)?;
    let input: Vec<Input> = serde_json::from_str(&input)?;
    let output: Vec<_> = input
        .iter()
        .map(|i| {
            let mut result = modifiers::translation::translate(
                &i.text,
                &i.kind,
                i.category.as_deref(),
                i.unscalable,
            );
            if let Some(modifier) = result.as_mut() {
                modifiers::apply_effect(modifier, i.effect);
            }
            result
        })
        .collect();
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
