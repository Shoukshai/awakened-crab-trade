#[allow(dead_code)]
#[path = "../src/widget_order.rs"]
mod widget_order;
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    order: Vec<widget_order::Key>,
    raises: Vec<widget_order::Key>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin()
        .take(16 * 1024 * 1024)
        .read_to_string(&mut text)?;
    let cases: Vec<Case> = serde_json::from_str(&text)?;
    let result: Vec<Vec<_>> = cases
        .into_iter()
        .map(|mut c| {
            c.raises
                .into_iter()
                .map(|k| {
                    widget_order::raise(&mut c.order, k);
                    c.order.clone()
                })
                .collect()
        })
        .collect();
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
