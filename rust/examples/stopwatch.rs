#[allow(dead_code)]
#[path = "../src/stopwatch.rs"]
mod stopwatch;
use serde::Deserialize;
#[derive(Deserialize)]
struct Step {
    at: u64,
    action: String,
    id: usize,
    interactive: bool,
    exclusive: bool,
    wanted: bool,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin()
        .take(16 * 1024 * 1024)
        .read_to_string(&mut input)?;
    let cases: Vec<Vec<Step>> = serde_json::from_str(&input)?;
    let mut result = Vec::new();
    for case in cases {
        let mut states = vec![stopwatch::State::default(); 3];
        let mut values = Vec::new();
        for step in case {
            let s = &mut states[step.id];
            match step.action.as_str() {
                "start" => s.start(step.at),
                "stop" => s.stop(step.at),
                "reset" => s.reset(step.at),
                "tick" => s.update(step.at),
                _ => return Err("unknown stopwatch action".into()),
            }
            values.push(serde_json::json!({"running":s.running,"millis":s.millis,"previous_tick":s.previous_tick,"invisible":s.invisible_on_blur,"formatted":stopwatch::State::formatted(s.millis),"visible":s.visible(step.wanted,step.interactive,step.exclusive)}));
        }
        result.push(values);
    }
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
