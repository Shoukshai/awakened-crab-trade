#[allow(dead_code)]
#[path = "../src/hotkeys.rs"]
mod hotkeys;
#[allow(dead_code)]
#[path = "../src/stash_scroll.rs"]
mod stash_scroll;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    ctrl: bool,
    active: bool,
    enabled: bool,
    x: f64,
    y: f64,
    rotation: i32,
    bounds: Option<stash_scroll::Bounds>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin()
        .take(16 * 1024 * 1024)
        .read_to_string(&mut text)?;
    let cases: Vec<Case> = serde_json::from_str(&text)?;
    let results:Vec<_>=cases.into_iter().map(|c|serde_json::json!({"inside":stash_scroll::is_stash_area(c.x,c.y,c.bounds),"action":stash_scroll::action(c.ctrl,c.active,c.enabled,c.x,c.y,c.rotation,c.bounds)})).collect();
    println!("{}", serde_json::to_string(&results)?);
    Ok(())
}
