//! Offline shortcut-name/keysym diagnostic for the unchanged upstream formatter.
#[allow(dead_code)]
#[path = "../src/hotkeys.rs"]
mod hotkeys;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .take(1024 * 1024)
        .read_to_string(&mut input)?;
    let values: Vec<String> = serde_json::from_str(&input)?;
    let output: Vec<_> = values
        .iter()
        .map(|s| {
            hotkeys::Hotkey::parse(s)
                .map(|key| serde_json::json!({"text":key.text(),"key":key.key,"keysym":key.keysym}))
        })
        .collect();
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}
