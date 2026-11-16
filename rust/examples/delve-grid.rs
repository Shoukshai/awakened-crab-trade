#[allow(dead_code)]
#[path = "../src/delve.rs"]
mod delve;
use serde::Deserialize;
#[derive(Deserialize)]
struct Input {
    viewports: Vec<[i32; 2]>,
    sequences: Vec<Vec<String>>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut text = String::new();
    std::io::stdin()
        .take(16 * 1024 * 1024)
        .read_to_string(&mut text)?;
    let input: Input = serde_json::from_str(&text)?;
    let geometry = input
        .viewports
        .into_iter()
        .map(|[w, h]| delve::Geometry::new(w, h))
        .collect::<Result<Vec<_>, _>>()?;
    let sequences = input
        .sequences
        .into_iter()
        .map(|sequence| {
            let mut p = delve::Policy::default();
            let mut exclusive = false;
            let mut active = false;
            sequence
                .into_iter()
                .map(|action| {
                    match action.as_str() {
                        "toggle" => {
                            active = false;
                            p.context(true, true, false, false, exclusive);
                            p.toggle(true, false);
                        }
                        "focus" => {
                            active = true;
                            p.context(true, false, false, true, exclusive);
                        }
                        "blur" => {
                            active = false;
                            p.context(true, true, false, false, exclusive);
                        }
                        "exclusive-on" => exclusive = true,
                        "exclusive-off" => exclusive = false,
                        "paint" => {
                            p.painted(p.generation);
                        }
                        "hide" => p.hide(delve::Reason::Explicit),
                        _ => return Err("Unknown fixture action"),
                    };
                    let visible = p.context(true, !active, false, active, exclusive);
                    Ok(serde_json::json!({"wanted":p.requested(),"visible":visible}))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    println!(
        "{}",
        serde_json::json!({"geometry":geometry,"sequences":sequences,"border_rgb":delve::BORDER_RGB})
    );
    Ok(())
}
