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
    #[serde(default)]
    exchangeable: bool,
    #[serde(default, rename = "tradeTag")]
    trade_tag: Option<String>,
    #[serde(default)]
    category: Option<String>,
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
            let mut entry: Identity = serde_json::from_str(line).unwrap();
            if entry.namespace == "AREA" {
                entry.name.push_str(" Map");
                entry.namespace = "MAP".into();
            }
            out.entry(key(&entry.name)).or_default().push(entry);
        }
        out
    })
}
fn identity(line: &Line) -> Option<&'static Identity> {
    let k = key(&line.text);
    identities().get(&k).and_then(|v| v.first()).or_else(|| {
        identities()
            .iter()
            .filter(|(name, v)| {
                let magic = line.rarity == Some(Rarity::Magic)
                    && !line.text.chars().any(|c| c.is_ascii_digit())
                    && v[0].craftable.is_some()
                    && v[0].namespace == "ITEM";
                name.len() >= 8
                    && k.find(name.as_str()).is_some_and(|start| {
                        magic || (start <= 2 && k.len() - start - name.len() <= 3)
                    })
            })
            .max_by_key(|(name, _)| name.len())
            .and_then(|(_, v)| v.first())
    })
}
fn select_item(lines: &[Line]) -> Result<(usize, &'static Identity), String> {
    let candidates: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| identity(l).map(|e| (i, e)))
        .collect();
    let mut ranked: Vec<_> = candidates
        .iter()
        .filter(|(i, e)| {
            !candidates.iter().any(|(j, u)| {
                u.namespace == "UNIQUE"
                    && j < i
                    && lines[*i].y - lines[*j].y < 70.
                    && u.unique.as_ref().and_then(|u| u.base.as_deref()) == Some(e.name.as_str())
            })
        })
        .map(|(i, e)| {
            let h = &lines[*i];
            let cx = h.x + h.width / 2.;
            let mut score = 0;
            let mut bottom = h.y + h.height;
            for l in lines
                .iter()
                .skip(*i + 1)
                .filter(|l| (l.x + l.width / 2. - cx).abs() < 80.)
            {
                if l.y - bottom > 95. {
                    break;
                }
                bottom = bottom.max(l.y + l.height);
                if l.text.contains(':') || stat(&l.text).is_some() {
                    score += 1;
                }
            }
            (*i, *e, score)
        })
        .collect();
    ranked.sort_by_key(|(_, _, score)| std::cmp::Reverse(*score));
    if let Some((i, e, score)) = ranked.first() {
        if ranked.len() == 1 || (*score > 0 && *score > ranked[1].2) {
            return Ok((*i, *e));
        }
    }
    Err("No unambiguous item tooltip found. Hover the item, keep Alt held until its details appear, then try Ctrl+D again (English game text).".into())
}
/// The scout pass finds catalogue text; all later OCR is restricted to that
/// tooltip's centered text column. No coordinates or manual region are needed.
pub fn tooltip_bounds(lines: &[Line]) -> Result<[f64; 4], String> {
    let mut sorted = lines.to_vec();
    sorted.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let (i, _) = select_item(&sorted)?;
    let h = &sorted[i];
    let center = h.x + h.width / 2.;
    let mut x = h.x;
    let mut right = h.x + h.width;
    let top = (h.y - 65.).max(0.);
    let mut bottom = h.y + h.height;
    for line in sorted
        .iter()
        .skip(i + 1)
        .filter(|l| (l.x + l.width / 2. - center).abs() < 90.)
    {
        if line.y - bottom > 95. {
            break;
        }
        x = x.min(line.x);
        right = right.max(line.x + line.width);
        bottom = bottom.max(line.y + line.height);
    }
    Ok([
        (x - 18.).max(0.),
        top,
        (right - x + 36.).max(220.),
        bottom - top + 20.,
    ])
}
pub fn draft(mut lines: Vec<Line>, engine: String) -> Result<Draft, String> {
    if lines.len() > 4096 || lines.iter().map(|l| l.text.len()).sum::<usize>() > 65536 {
        return Err(
            "Too much text in the capture. Hover a single item tooltip and try again.".into(),
        );
    }
    lines.retain(|l| {
        !l.text.trim().is_empty() && [l.x, l.y, l.width, l.height].iter().all(|v| v.is_finite())
    });
    lines.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let (index, entry) = select_item(&lines)?;
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
        "ITEM"
            if entry.craftable.is_none() && (entry.exchangeable || entry.trade_tag.is_some()) =>
        {
            Rarity::Currency
        }
        "DIVINATION_CARD" => Rarity::Divination,
        _ => header
            .rarity
            .clone()
            .or_else(|| previous.and_then(|l| l.rarity.clone()))
            .unwrap_or(Rarity::Normal),
    };
    let named = matches!(rarity, Rarity::Rare | Rarity::Unique);
    let name = if entry.namespace == "MAP" && !named {
        "Map".into()
    } else if rarity == Rarity::Unique {
        entry.name.clone()
    } else if named {
        previous
            .map(|l| l.text.clone())
            .unwrap_or_else(|| entry.name.clone())
    } else {
        entry.name.clone()
    };
    let base = if entry.namespace == "MAP" {
        "Map".into()
    } else {
        entry
            .unique
            .as_ref()
            .and_then(|u| u.base.clone())
            .unwrap_or_else(|| entry.name.clone())
    };
    let class = if entry.namespace == "MAP"
        || entry.category.as_deref() == Some("Map")
        || entry.craftable.as_ref().and_then(|c| c.category.as_deref()) == Some("Map")
    {
        "Maps".into()
    } else {
        entry
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
            })
    };
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
    if let Some(tier) = key(&header.text)
        .strip_prefix("maptier")
        .and_then(|v| v.parse::<u32>().ok())
    {
        text.push_str(&format!("Map Tier: {tier}\n"));
    }
    if entry.namespace == "MAP" {
        text.push_str(&format!(
            "Map Area: {}\n",
            entry.name.trim_end_matches(" Map")
        ));
    }
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
        assert_eq!(d.engine, "test");
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
    #[test]
    fn all_tooltip_families_resolve_without_coordinates() {
        let cases = [
            (
                vec![
                    line("Impale Support", 80., None),
                    line("Level: 20", 110., None),
                    line("Quality: +20%", 140., None),
                ],
                crate::item::Family::Gem,
            ),
            (
                vec![
                    line("Divine Orb", 80., None),
                    line("Stack Size: 1/20", 110., None),
                ],
                crate::item::Family::Currency,
            ),
            (
                vec![
                    line("Goldrim", 80., Some(Rarity::Unique)),
                    line("Leather Cap", 105., Some(Rarity::Unique)),
                    line("Item Level: 71", 135., None),
                    line("+35% to all Elemental Resistances", 165., None),
                ],
                crate::item::Family::Unique,
            ),
            (
                vec![
                    line("Strand Map", 80., Some(Rarity::Normal)),
                    line("Map Tier: 16", 110., None),
                    line("Item Level: 83", 140., None),
                ],
                crate::item::Family::Map,
            ),
        ];
        for (lines, family) in cases {
            let bounds = tooltip_bounds(&lines).unwrap();
            assert!(bounds[2] > 0. && bounds[3] > 0.);
            let d = draft(lines, "test".into()).unwrap();
            let item = crate::item::Item::parse(&d.text).unwrap();
            assert_eq!(item.family(), family, "{}", d.text);
        }
    }
    #[test]
    fn a_magic_name_can_identify_its_exact_catalogue_base() {
        let d = draft(
            vec![
                line(
                    "GLYPHIC DRIFTWOOD WAND OF THE SCHOLAR",
                    20.,
                    Some(Rarity::Magic),
                ),
                line("Item Level: 71", 50., None),
                line("+25 to maximum Mana", 80., None),
            ],
            "test".into(),
        )
        .unwrap();
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(i.base, "Driftwood Wand");
        assert_eq!(i.rarity, Rarity::Magic);
    }
}
