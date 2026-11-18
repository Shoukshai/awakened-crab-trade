//! Positioned fragments and independent readings, without fuzzy stat edits.
use super::{key, modifier_header, numbers, stat, stat_key, Line, Reading};
use regex::Regex;
use std::sync::OnceLock;

pub(super) const PROPERTIES: &[&str] = &[
    "Item Level",
    "Quality",
    "Level",
    "Map Tier",
    "Armour",
    "Evasion Rating",
    "Energy Shield",
    "Ward",
    "Chance to Block",
    "Physical Damage",
    "Elemental Damage",
    "Critical Strike Chance",
    "Attacks per Second",
    "Stack Size",
];

pub(super) fn property(text: &str) -> Option<(&'static str, String)> {
    let (label, value) = text.split_once(':')?;
    let name = *PROPERTIES.iter().find(|p| key(p) == key(label))?;
    let value = value.trim();
    if ["Armour", "Evasion Rating", "Energy Shield", "Ward"].contains(&name)
        && ranges().is_match(value)
        && super::base_roll(value).is_none()
    {
        return None;
    }
    if name == "Item Level" {
        value.parse::<u32>().ok().filter(|v| *v <= 100)?;
    } else {
        static VALUE: OnceLock<Regex> = OnceLock::new();
        if !VALUE
            .get_or_init(|| {
                Regex::new(r"(?i)^[+-]?\d[\d.,+/%() –-]*(?:\((?:max|augmented)\))?$").unwrap()
            })
            .is_match(value)
        {
            return None;
        }
    }
    Some((name, value.into()))
}

fn same_row(a: &Line, b: &Line) -> bool {
    (a.y + a.height / 2. - b.y - b.height / 2.).abs() < a.height.min(b.height).max(8.) * 0.6
}

/// Reattach only an explicit property label and its unique right-hand value on
/// the same baseline. A nearby requirement level is never an ilvl substitute.
pub fn repair_properties(lines: &mut Vec<Line>) {
    // Requirements can also be split by Windows OCR. Remove only attribute/
    // number fragments on an explicitly labelled requirement baseline.
    let requirements: Vec<_> = lines
        .iter()
        .filter(|l| key(&l.text).starts_with("requires"))
        .cloned()
        .collect();
    lines.retain(|l| {
        let words: Vec<_> = l
            .text
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        let fragment = !words.is_empty()
            && words.iter().all(|w| {
                w.chars().all(|c| c.is_ascii_digit())
                    || ["str", "dex", "int", "gem", "level"]
                        .contains(&w.to_ascii_lowercase().as_str())
            });
        !fragment
            || !requirements
                .iter()
                .any(|r| same_row(r, l) && l.x >= r.x + r.width - 2.)
    });
    let mut i = 0;
    while i < lines.len() {
        let Some(name) = PROPERTIES
            .iter()
            .find(|p| key(p) == key(&lines[i].text))
            .copied()
        else {
            i += 1;
            continue;
        };
        let a = &lines[i];
        let candidates: Vec<_> = lines
            .iter()
            .enumerate()
            .filter(|(j, b)| {
                *j != i
                    && same_row(a, b)
                    && b.x >= a.x + a.width - 2.
                    && b.x - a.x - a.width <= a.height.max(b.height) * 6.
                    && property(&format!("{name}: {}", b.text)).is_some()
            })
            .map(|(j, _)| j)
            .collect();
        if let [j] = candidates.as_slice() {
            let b = lines[*j].clone();
            let a = &mut lines[i];
            a.text = format!("{name}: {}", b.text.trim());
            a.width = b.x + b.width - a.x;
            a.height = a.height.max(b.y + b.height - a.y);
            lines.remove(*j);
            if *j < i {
                i -= 1;
            }
        }
        i += 1;
    }
}

fn canonical(text: &str) -> Option<String> {
    stat(text).or_else(|| property(text).map(|(p, v)| format!("{p}: {v}")))
}
fn ranges() -> &'static Regex {
    static RANGE: OnceLock<Regex> = OnceLock::new();
    RANGE.get_or_init(|| Regex::new(r"\([+-]?\d+(?:\.\d+)?\s*[-–]\s*[+-]?\d+(?:\.\d+)?\)").unwrap())
}
fn values(text: &str) -> Vec<f64> {
    numbers()
        .find_iter(text)
        .filter_map(|m| m.as_str().parse().ok())
        .collect()
}
fn equivalent(a: &str, b: &str) -> bool {
    let a = a.trim_end_matches(" — Unscalable Value");
    let b = b.trim_end_matches(" — Unscalable Value");
    let aa = ranges().replace_all(a, "");
    let bb = ranges().replace_all(b, "");
    if stat_key(&aa) != stat_key(&bb) || values(&aa) != values(&bb) {
        return false;
    }
    let ar: Vec<_> = ranges().find_iter(a).map(|m| values(m.as_str())).collect();
    let br: Vec<_> = ranges().find_iter(b).map(|m| values(m.as_str())).collect();
    ar.is_empty() || br.is_empty() || ar == br
}

/// Each pass gets at most one vote on a physical row. A conflicting roll needs
/// two agreeing readings; adding a range to the same roll is not a conflict.
pub fn merge_stats(lines: &mut Vec<Line>, mut extra: Vec<Line>, pass: u8) {
    repair_properties(lines);
    repair_properties(&mut extra);
    for mut line in extra {
        if let Some((kind, header)) = modifier_header(&line.text) {
            let nearby = lines.iter().position(|l| {
                same_row(l, &line) && (l.x + l.width / 2. - line.x - line.width / 2.).abs() < 90.
            });
            if let Some(i) = nearby {
                let old = modifier_header(&lines[i].text);
                let generic = old.as_ref().is_some_and(|(k, h)| {
                    k == "explicit" && !h.contains("Prefix") && !h.contains("Suffix")
                });
                if old.is_none() || (generic && kind != "explicit") {
                    lines[i] = line;
                }
            } else {
                line.text = header;
                lines.push(line);
            }
            continue;
        }
        let Some(value) = canonical(&line.text) else {
            continue;
        };
        let nearby = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                same_row(l, &line) && (l.x + l.width / 2. - line.x - line.width / 2.).abs() < 90.
            })
            .min_by(|(_, a), (_, b)| {
                (a.x + a.width / 2. - line.x - line.width / 2.)
                    .abs()
                    .total_cmp(&(b.x + b.width / 2. - line.x - line.width / 2.).abs())
            })
            .map(|(i, _)| i);
        let Some(i) = nearby else {
            line.readings = vec![Reading { pass, text: value }];
            lines.push(line);
            continue;
        };
        let current = &mut lines[i];
        if current.readings.is_empty() {
            if let Some(old) = canonical(&current.text) {
                current.readings.push(Reading { pass: 0, text: old });
            }
        }
        if current.readings.iter().any(|r| r.pass == pass) {
            continue;
        }
        current.readings.push(Reading { pass, text: value });
        let best = current
            .readings
            .iter()
            .max_by_key(|r| {
                current
                    .readings
                    .iter()
                    .filter(|s| equivalent(&r.text, &s.text))
                    .count()
            })
            .unwrap();
        let agreeing: Vec<_> = current
            .readings
            .iter()
            .filter(|r| equivalent(&r.text, &best.text))
            .collect();
        let coherent = agreeing
            .iter()
            .all(|a| agreeing.iter().all(|b| equivalent(&a.text, &b.text)));
        if coherent && (agreeing.len() == current.readings.len() || agreeing.len() >= 2) {
            let text = agreeing
                .iter()
                .max_by_key(|r| {
                    (
                        ranges().find_iter(&r.text).count(),
                        r.text.ends_with(" — Unscalable Value"),
                    )
                })
                .unwrap()
                .text
                .clone();
            let evidence = std::mem::take(&mut current.readings);
            *current = line;
            current.text = text;
            current.readings = evidence;
        } else {
            current.text = format!(
                "Conflicting OCR readings: {}",
                current
                    .readings
                    .iter()
                    .map(|r| r.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" / ")
            );
            current.source = None;
        }
    }
}

pub fn needs_retry(lines: &[Line]) -> bool {
    super::draft(lines.to_vec(), "probe".into()).is_ok_and(|d| !d.unknown.is_empty())
}
