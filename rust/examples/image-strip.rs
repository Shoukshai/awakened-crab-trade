#[allow(dead_code)]
#[path = "../src/image_strip.rs"]
mod image_strip;
use serde::Deserialize;
#[derive(Deserialize)]
struct HoverStep {
    action: String,
    disabled: bool,
    rect: [f64; 4],
    position: [f64; 2],
    movement: [f64; 2],
}
#[derive(Deserialize)]
struct Input {
    hover: Vec<Vec<HoverStep>>,
    origins: Vec<(image_strip::Config, [f64; 4])>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin()
        .take(16 * 1024 * 1024)
        .read_to_string(&mut text)?;
    let input: Input = serde_json::from_str(&text)?;
    let hover: Vec<Vec<_>> = input
        .hover
        .into_iter()
        .map(|case| {
            let mut h = image_strip::Hover::default();
            case.into_iter()
                .map(|s| {
                    match s.action.as_str() {
                        "move" => h.motion(s.disabled, s.rect, s.position, s.movement),
                        "leave" => h.leave(),
                        "click" => h.click(s.disabled),
                        _ => panic!("unknown hover fixture"),
                    };
                    serde_json::to_value(&h).unwrap()
                })
                .collect()
        })
        .collect();
    let origins: Vec<_> = input
        .origins
        .into_iter()
        .map(|(c, [w, h, ww, wh])| c.origin(w, h, ww, wh))
        .collect();
    println!("{}", serde_json::json!({"hover":hover,"origins":origins}));
    Ok(())
}
