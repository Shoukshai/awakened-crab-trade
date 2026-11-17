#[allow(dead_code)]
#[path = "../src/hotkeys.rs"]
mod hotkeys;
#[allow(dead_code)]
#[path = "../src/stash_search.rs"]
mod stash_search;
#[allow(dead_code)]
#[path = "../src/widget_menu.rs"]
mod widget_menu;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    entries: Vec<stash_search::Entry>,
    operations: Vec<Option<u64>>,
}
#[derive(Deserialize)]
struct Input {
    cases: Vec<Case>,
    random: Vec<[f64; 2]>,
    names: Vec<String>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin()
        .take(8 * 1024 * 1024)
        .read_to_string(&mut text)?;
    let input: Input = serde_json::from_str(&text)?;
    let cases: Vec<_> = input
        .cases
        .into_iter()
        .map(|case| {
            let mut c = stash_search::Config {
                entries: case.entries,
                ..Default::default()
            };
            case.operations
                .into_iter()
                .map(|op| {
                    if let Some(id) = op {
                        c.entries.retain(|e| e.id != id);
                    } else {
                        c.add_entry().unwrap();
                    }
                    c.entries.clone()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let new: Vec<_> = input
        .random
        .into_iter()
        .map(|[x, y]| stash_search::Config::new(7, x, y))
        .collect();
    let labels: Vec<_> = input
        .names
        .iter()
        .map(|n| {
            stash_search::Entry {
                id: 1,
                name: n.clone(),
                text: "Currency".into(),
                hotkey: None,
            }
            .label()
            .to_owned()
        })
        .collect();
    let similar: Vec<_> = input
        .names
        .iter()
        .map(|n| stash_search::similar_filter(n))
        .collect();
    println!(
        "{}",
        serde_json::json!({"defaults":stash_search::defaults(),"new":new,"operations":cases,"labels":labels,"similar":similar})
    );
    Ok(())
}
