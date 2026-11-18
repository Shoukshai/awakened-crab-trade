//! Conservative conversion of positioned OCR lines to reviewable item text.
//! No fuzzy name/stat replacement and no inferred item level.
use crate::item::Rarity;
use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Line {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rarity: Option<Rarity>,
}
#[derive(Debug)]
pub struct Draft {
    pub text: String,
    pub unknown: Vec<String>,
    pub engine: String,
}
pub fn key(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}
fn numbers() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"-?\d+(?:\.\d+)?").unwrap())
}
fn stat_key(s: &str) -> String {
    numbers()
        .replace_all(&s.to_ascii_lowercase(), "#")
        .chars()
        .filter(|c| c.is_ascii_alphabetic() || *c == '#' || *c == '%')
        .collect()
}
/// Recover canonical capitalization/spacing only when the entire stat matches.
fn stat(s: &str) -> Option<String> {
    static MATCHERS: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    let matchers = MATCHERS.get_or_init(|| {
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        for line in include_str!("data/base-stats.ndjson")
            .lines()
            .chain(include_str!("data/current-stats.ndjson").lines())
        {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            if let Some(a) = v["matchers"].as_array() {
                for m in a {
                    if let Some(s) = m["string"].as_str().filter(|s| !s.contains('\n')) {
                        let list = out.entry(stat_key(s)).or_default();
                        if !list.iter().any(|v| v == s) {
                            list.push(s.into());
                        }
                    }
                }
            }
        }
        out
    });
    let text = s.replace('−', "-").replace('’', "'");
    static RANGE: OnceLock<Regex> = OnceLock::new();
    let range = RANGE
        .get_or_init(|| Regex::new(r"\([+-]?\d+(?:\.\d+)?\s*[-–]\s*[+-]?\d+(?:\.\d+)?\)").unwrap());
    let text = range.replace_all(&text, "");
    let values: Vec<_> = numbers().find_iter(&text).map(|m| m.as_str()).collect();
    let mut results = Vec::new();
    static TOKENS: OnceLock<Regex> = OnceLock::new();
    let tokens = TOKENS.get_or_init(|| Regex::new(r"#|-?\d+(?:\.\d+)?").unwrap());
    for template in matchers.get(&stat_key(&text))? {
        let parts: Vec<_> = tokens.find_iter(template).collect();
        if parts.len() != values.len() {
            continue;
        }
        if parts
            .iter()
            .zip(&values)
            .any(|(p, v)| p.as_str() != "#" && p.as_str() != *v)
        {
            continue;
        }
        let mut result = String::new();
        let mut offset = 0;
        for (part, value) in parts.iter().zip(&values) {
            result.push_str(&template[offset..part.start()]);
            result.push_str(value);
            offset = part.end();
        }
        result.push_str(&template[offset..]);
        let result = result.trim().to_string();
        if !results.contains(&result) {
            results.push(result);
        }
    }
    (results.len() == 1).then(|| results.remove(0))
}
#[derive(serde::Deserialize)]
struct Identity {
    name: String,
    namespace: String,
    craftable: Option<Category>,
    unique: Option<Unique>,
}
#[derive(serde::Deserialize)]
struct Category {
    category: Option<String>,
}
#[derive(serde::Deserialize)]
struct Unique {
    base: Option<String>,
}
fn identities() -> &'static HashMap<String, Vec<Identity>> {
    static ITEMS: OnceLock<HashMap<String, Vec<Identity>>> = OnceLock::new();
    ITEMS.get_or_init(|| {
        let mut out: HashMap<String, Vec<Identity>> = HashMap::new();
        for line in include_str!("data/base-items.ndjson")
            .lines()
            .chain(include_str!("data/current-items.ndjson").lines())
        {
            let entry: Identity = serde_json::from_str(line).unwrap();
            if entry.namespace != "AREA" {
                out.entry(key(&entry.name)).or_default().push(entry);
            }
        }
        out
    })
}
pub fn draft(mut lines: Vec<Line>, engine: String) -> Result<Draft, String> {
    if lines.len() > 4096 || lines.iter().map(|l| l.text.len()).sum::<usize>() > 65536 {
        return Err("Too much text in the OCR area. Select a smaller tooltip area.".into());
    }
    lines.retain(|l| {
        !l.text.trim().is_empty() && [l.x, l.y, l.width, l.height].iter().all(|v| v.is_finite())
    });
    lines.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let matches: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let k = key(&l.text);
            let exact = identities().get(&k).and_then(|v| v.first());
            // OCR sometimes appends a tiny gilded corner ornament to the base line.
            // Accept an exact catalogue prefix with <= 3 trailing glyphs, never a
            // substring inside a sales-history line or an edit-distance correction.
            let entry = exact.or_else(|| {
                identities()
                    .iter()
                    .filter(|(name, _)| {
                        name.len() >= 8
                            && k.find(name.as_str()).is_some_and(|start| {
                                start <= 2 && k.len() - start - name.len() <= 3
                            })
                    })
                    .max_by_key(|(name, _)| name.len())
                    .and_then(|(_, v)| v.first())
            });
            entry.map(|e| (i, e))
        })
        .collect();
    // Multiple named items in inventory/history must not silently select one.
    let (index, entry) = matches.iter().find(|(_, e)| e.namespace == "UNIQUE")
        .copied().or_else(|| (matches.len()==1).then(|| matches[0]))
        .ok_or("No unambiguous item base found. Set a smaller OCR area around the tooltip and try again (English game text).")?;
    let header = &lines[index];
    let center = header.x + header.width / 2.;
    let in_column = |l: &Line| (l.x + l.width / 2. - center).abs() < (header.width * 0.4).max(42.);
    let previous = lines[..index]
        .iter()
        .rev()
        .find(|l| in_column(l) && header.y - l.y < 70.);
    let rarity = match entry.namespace.as_str() {
        "UNIQUE" => Rarity::Unique,
        "GEM" => Rarity::Gem,
        "CURRENCY" => Rarity::Currency,
        "DIVINATION_CARD" => Rarity::Divination,
        _ => previous
            .and_then(|l| l.rarity.clone())
            .or(header.rarity.clone())
            .unwrap_or(Rarity::Normal),
    };
    let named = matches!(rarity, Rarity::Rare | Rarity::Unique);
    let name = if rarity == Rarity::Unique {
        entry.name.clone()
    } else if named {
        previous
            .map(|l| l.text.clone())
            .unwrap_or_else(|| entry.name.clone())
    } else {
        entry.name.clone()
    };
    let base = entry
        .unique
        .as_ref()
        .and_then(|u| u.base.clone())
        .unwrap_or_else(|| entry.name.clone());
    let class = entry
        .craftable
        .as_ref()
        .and_then(|c| c.category.clone())
        .unwrap_or_else(|| {
            match rarity {
                Rarity::Gem => "Skill Gems",
                Rarity::Currency => "Stackable Currency",
                Rarity::Divination => "Divination Cards",
                _ => "Items",
            }
            .into()
        });
    let mut text = format!(
        "Item Class: {class}\nRarity: {}\n{name}\n",
        match rarity {
            Rarity::Normal => "Normal",
            Rarity::Magic => "Magic",
            Rarity::Rare => "Rare",
            Rarity::Unique => "Unique",
            Rarity::Gem => "Gem",
            Rarity::Currency => "Currency",
            Rarity::Divination => "Divination Card",
        }
    );
    if named {
        text.push_str(&format!("{base}\n"));
    }
    text.push_str("--------\n");
    let mut unknown = Vec::new();
    let mut last_y = header.y + header.height;
    let properties = [
        "Item Level",
        "Quality",
        "Level",
        "Map Tier",
        "Armour",
        "Evasion Rating",
        "Energy Shield",
        "Physical Damage",
        "Elemental Damage",
        "Critical Strike Chance",
        "Attacks per Second",
        "Stack Size",
    ];
    let mut kind = "explicit";
    for line in lines.iter().skip(index + 1).filter(|l| in_column(l)) {
        if line.y - last_y > 95. {
            break;
        }
        last_y = line.y + line.height;
        let k = key(&line.text);
        if k.contains("modifier") {
            if k.contains("implicit") {
                kind = "implicit";
                continue;
            }
            if k.contains("crafted") {
                kind = "crafted";
                continue;
            }
            if k.contains("fractured") {
                kind = "fractured";
                continue;
            }
            if k.starts_with("prefix") || k.starts_with("suffix") || k.starts_with("explicit") {
                kind = "explicit";
                continue;
            }
        }
        if k.starts_with("requires") || k.starts_with("tier") {
            continue;
        }
        if k == "corrupted" || k == "mirrored" || k == "unidentified" {
            text.push_str(match k.as_str() {
                "corrupted" => "--------\nCorrupted\n",
                "mirrored" => "--------\nMirrored\n",
                _ => "--------\nUnidentified\n",
            });
            continue;
        }
        if let Some(property) = properties
            .iter()
            .find(|p| k.starts_with(&key(p)) && line.text.contains(':'))
        {
            let value = line.text.split_once(':').unwrap().1.trim();
            if *property == "Item Level" && !value.parse::<u32>().is_ok_and(|v| v <= 100) {
                unknown.push(line.text.clone());
                continue;
            }
            // Only explicit Item Level, never "Requires Level" or a tier.
            text.push_str(&format!("{property}: {value}\n"));
            continue;
        }
        if let Some(s) = stat(&line.text) {
            text.push_str(&format!(
                "{{ {} Modifier }}\n{s}\n",
                match kind {
                    "implicit" => "Implicit",
                    "crafted" => "Crafted",
                    "fractured" => "Fractured",
                    _ => "Explicit",
                }
            ));
        } else {
            unknown.push(line.text.clone());
        }
    }
    for line in &unknown {
        text.push_str(&format!("{{ Unrecognized OCR }}\n{line}\n"));
    }
    Ok(Draft {
        text,
        unknown,
        engine,
    })
}

/// Re-run exact stat normalization after the user edits an uncertain OCR line.
pub fn normalize_review(text: &str) -> Result<String, String> {
    if text.len() > 65536 || text.lines().count() > 256 {
        return Err("OCR review text is too large.".into());
    }
    Ok(text
        .lines()
        .map(|line| stat(line).unwrap_or_else(|| line.to_string()))
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(text: &str, y: f64, rarity: Option<Rarity>) -> Line {
        Line {
            text: text.into(),
            x: 800.,
            y,
            width: 300.,
            height: 20.,
            rarity,
        }
    }
    #[test]
    fn screenshot_gloves_keep_real_mods_and_never_infer_ilvl_from_requirements() {
        let lines = vec![
            line("VICTORY TOUCH", 0., Some(Rarity::Rare)),
            line("ASSASSIN'S MITTS", 22., None),
            line("QUALITY: +14%", 44., None),
            line("REQUIRES LEVEL 61 (GEM), 42 STR (GEM)", 66., None),
            line("ADDS 1 TO 2 PHYSICAL DAMAGE TO ATTACKS", 88., None),
            line("+119 TO MAXIMUM LIFE", 110., None),
            line("REGENERATE 56.5 LIFE PER SECOND", 132., None),
            line("+35% TO FIRE RESISTANCE", 154., None),
        ];
        let d = draft(lines, "test".into()).unwrap();
        let item = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(item.base, "Assassin's Mitts");
        assert_eq!(item.rarity, Rarity::Rare);
        assert_eq!(item.item_level, None);
        assert!(item.modifiers.len() >= 4, "{} {:?}", d.text, d.unknown);
        assert!(d.text.contains("Regenerate 56.5 Life per second"));
    }
    #[test]
    fn only_literal_item_level_is_used_and_unknown_stats_are_not_guessed() {
        let d = draft(
            vec![
                line("MORBID SHROUD", 0., Some(Rarity::Rare)),
                line("VARNISHED COAT", 22., None),
                line("ITEM LEVEL: 80", 44., None),
                line("+32% TO CHAOS RESISTANCE", 66., None),
                line("+119 TO MAXIMUM L1FE", 88., None),
                line("CORRUPTED", 110., None),
            ],
            "test".into(),
        )
        .unwrap();
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(i.item_level, Some(80));
        assert!(i.corrupted);
        assert_eq!(d.unknown, vec!["+119 TO MAXIMUM L1FE"]);
        assert_eq!(i.modifiers.len(), 1);
    }
    #[test]
    fn ambiguous_inventory_is_rejected() {
        assert!(draft(
            vec![
                line("Driftwood Wand", 0., None),
                line("Varnished Coat", 50., None)
            ],
            "test".into()
        )
        .is_err());
    }
    #[test]
    fn advanced_implicit_headers_do_not_leak_into_explicit_mods() {
        let d = draft(
            vec![
                line("Driftwood Wand", 0., None),
                line("Item Level: 71", 22., None),
                line("Implicit Modifier", 44., None),
                line("8% increased Spell Damage", 66., None),
                line("Prefix Modifier Life (Tier: 1)", 88., None),
                line("+100 to maximum Mana", 110., None),
            ],
            "test".into(),
        )
        .unwrap();
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert!(i.modifiers.iter().any(|m| m.kind == "implicit"));
        assert!(i.modifiers.iter().any(|m| m.kind == "explicit"));
        assert_eq!(i.item_level, Some(71));
    }
    #[test]
    fn decorative_header_glyphs_do_not_hide_an_exact_base() {
        let d = draft(
            vec![
                line("Morbid Shroud", 0., Some(Rarity::Rare)),
                line("i VARNISHED COAT cal", 22., Some(Rarity::Rare)),
                line("Corrupted", 44., None),
            ],
            "test".into(),
        )
        .unwrap();
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(i.base, "Varnished Coat");
        assert!(i.corrupted);
    }
    #[test]
    fn correcting_an_ocr_line_recovers_the_stat_without_guessing_its_spelling() {
        let d = draft(
            vec![
                line("Varnished Coat", 0., Some(Rarity::Rare)),
                line("123% INCREASED EVASION AND ENERGY SHIELD G", 22., None),
            ],
            "test".into(),
        )
        .unwrap();
        let bad = crate::item::Item::parse(&d.text).unwrap();
        assert!(bad.modifiers.is_empty());
        assert_eq!(d.unknown.len(), 1);
        let corrected = d.text.replace("SHIELD G", "SHIELD");
        let text = normalize_review(&corrected).unwrap();
        let fixed = crate::item::Item::parse(&text).unwrap();
        assert_eq!(fixed.modifiers.len(), 1);
    }
}
