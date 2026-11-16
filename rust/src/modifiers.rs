//! Conservative first stat-translation port. Only unambiguous single-roll
//! matchers are offered. Unsupported/multiline/option stats stay unselected.
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modifier {
    pub text: String,
    pub id: String,
    pub value: f64,
}
struct Matcher {
    text: String,
    ids: HashMap<String, Vec<String>>,
    negate: bool,
}
fn numbers() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[+-]?\d+(?:\.\d+)?").unwrap())
}
fn skeleton(text: &str) -> String {
    numbers().replace_all(text, "#").into_owned()
}
fn index() -> &'static HashMap<String, Vec<Matcher>> {
    static INDEX: OnceLock<HashMap<String, Vec<Matcher>>> = OnceLock::new();
    INDEX.get_or_init(|| {
        fn add(v: &Value, out: &mut HashMap<String, Vec<Matcher>>) {
            if let Some(children) = v["stats"].as_array() {
                for child in children {
                    add(child, out);
                }
            }
            let Some(matchers) = v["matchers"].as_array() else {
                return;
            };
            for m in matchers {
                let Some(text) = m["string"].as_str() else {
                    continue;
                };
                if text.matches('#').count() != 1
                    || text.contains('\n')
                    || m.get("advanced").is_some()
                {
                    continue;
                }
                let Ok(ids) = serde_json::from_value(v["trade"]["ids"].clone()) else {
                    continue;
                };
                out.entry(skeleton(text)).or_default().push(Matcher {
                    text: text.into(),
                    ids,
                    negate: m["negate"].as_bool().unwrap_or(false),
                });
            }
        }
        let mut out = HashMap::new();
        for line in include_str!("../../renderer/public/data/en/stats.ndjson").lines() {
            add(
                &serde_json::from_str::<Value>(line).expect("upstream stat entry"),
                &mut out,
            );
        }
        out
    })
}
pub fn parse(raw: &str) -> Vec<Modifier> {
    static RANGES: OnceLock<Regex> = OnceLock::new();
    let ranges =
        RANGES.get_or_init(|| Regex::new(r"\([+-]?\d+(?:\.\d+)?-[+-]?\d+(?:\.\d+)?\)").unwrap());
    let mut kind = "explicit";
    let mut result = Vec::new();
    for raw_line in raw.lines() {
        let raw_line = raw_line.trim();
        if raw_line == "--------" {
            kind = "explicit";
            continue;
        }
        if raw_line.starts_with('{') {
            kind = if raw_line.contains("Implicit Modifier") {
                "implicit"
            } else if raw_line.contains("Crafted") {
                "crafted"
            } else {
                "explicit"
            };
            continue;
        }
        let (line, line_kind) = if let Some(s) = raw_line.strip_suffix(" (implicit)") {
            (s, "implicit")
        } else if let Some(s) = raw_line.strip_suffix(" (crafted)") {
            (s, "crafted")
        } else if let Some(s) = raw_line.strip_suffix(" (fractured)") {
            (s, "fractured")
        } else if let Some(s) = raw_line.strip_suffix(" (enchant)") {
            (s, "enchant")
        } else {
            (raw_line, kind)
        };
        let line = ranges.replace_all(line, "");
        let key = skeleton(&line);
        let Some(candidates) = index().get(&key) else {
            continue;
        };
        let mut found = Vec::new();
        for m in candidates {
            let Some(ids) = m.ids.get(line_kind) else {
                continue;
            };
            let pattern = format!(
                "^{}$",
                m.text
                    .split('#')
                    .map(regex::escape)
                    .collect::<Vec<_>>()
                    .join(r"([+-]?\d+(?:\.\d+)?)")
            );
            let Ok(regex) = Regex::new(&pattern) else {
                continue;
            };
            let Some(caps) = regex.captures(&line) else {
                continue;
            };
            let Ok(mut value) = caps[1].parse::<f64>() else {
                continue;
            };
            if m.negate {
                value = -value;
            }
            for id in ids.iter().filter(|id| !id.contains('|')) {
                found.push((id.clone(), value));
            }
        }
        let identities: HashSet<_> = found.iter().map(|(id, v)| (id, v.to_bits())).collect();
        if identities.len() == 1 {
            let (id, value) = found.remove(0);
            result.push(Modifier {
                text: raw_line.into(),
                id,
                value,
            });
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_advanced_copied_wand_keeps_implicit_roll() {
        let mods = parse("Item Class: Wands\n--------\n{ Implicit Modifier — Damage, Caster }\n8(8-12)% increased Spell Damage");
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].id, "implicit.stat_2974417149");
        assert_eq!(mods[0].value, 8.);
    }
    #[test]
    fn explicit_life_and_crafted_resistance_keep_their_kind() {
        let mods = parse("+73 to maximum Life\n+32% to Fire Resistance (crafted)");
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].id, "explicit.stat_3299347043");
        assert_eq!(mods[1].id, "crafted.stat_3372524247");
    }
    #[test]
    fn unknown_text_does_not_create_filters() {
        assert!(parse("unrecognized mod 99").is_empty());
    }
}
