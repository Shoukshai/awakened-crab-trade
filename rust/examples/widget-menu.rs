#[allow(dead_code)]
#[path = "../src/widget_menu.rs"]
mod widget_menu;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    items: Vec<widget_menu::Entry>,
    always: bool,
}
#[derive(Deserialize)]
struct Input {
    cases: Vec<Case>,
    origins: Vec<(widget_menu::Config, [f64; 4])>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin()
        .take(16 * 1024 * 1024)
        .read_to_string(&mut text)?;
    let input: Input = serde_json::from_str(&text)?;
    let labels: Vec<_> = input
        .cases
        .into_iter()
        .map(|c| widget_menu::entries(&c.items, c.always))
        .collect();
    let origins: Vec<_> = input
        .origins
        .into_iter()
        .map(|(c, [w, h, ww, wh])| c.origin(w, h, ww, wh))
        .collect();
    println!(
        "{}",
        serde_json::json!({"labels":labels,"origins":origins,"default":widget_menu::Config::default()})
    );
    Ok(())
}
