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
    /// Color evidence for special modifier sources, independent of faint labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
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
            // Trade pseudo filters include the eldritch source headers. They
            // are metadata, not additional physical lines on the item.
            if v["trade"]["ids"]
                .as_object()
                .is_some_and(|ids| !ids.is_empty() && ids.keys().all(|kind| kind == "pseudo"))
            {
                continue;
            }
            if let Some(a) = v["matchers"].as_array() {
                for m in a {
                    if let Some(s) = m["string"].as_str() {
                        for s in s.lines() {
                            let list = out.entry(stat_key(s)).or_default();
                            if !list.iter().any(|v| v == s) {
                                list.push(s.into());
                            }
                        }
                    }
                }
            }
        }
        out
    });
    let original = s.replace('−', "-").replace('’', "'");
    static RANGE: OnceLock<Regex> = OnceLock::new();
    let range = RANGE
        .get_or_init(|| Regex::new(r"\([+-]?\d+(?:\.\d+)?\s*[-–]\s*[+-]?\d+(?:\.\d+)?\)").unwrap());
    let text = range.replace_all(&original, "");
    // Keep the observed roll bounds for the native parser and review screen.
    static ROLLS: OnceLock<Regex> = OnceLock::new();
    let rolls = ROLLS.get_or_init(|| {
        Regex::new(r"(-?\d+(?:\.\d+)?)(\([+-]?\d+(?:\.\d+)?\s*[-–]\s*[+-]?\d+(?:\.\d+)?\))?")
            .unwrap()
    });
    let values: Vec<_> = rolls.captures_iter(&original).collect();
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
            .any(|(p, v)| p.as_str() != "#" && p.as_str() != &v[1])
        {
            continue;
        }
        let mut result = String::new();
        let mut offset = 0;
        for (part, value) in parts.iter().zip(&values) {
            result.push_str(&template[offset..part.start()]);
            result.push_str(&value[1]);
            if let Some(bounds) = value.get(2) {
                result.push_str(&bounds.as_str().replace('–', "-").replace(' ', ""));
            }
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

/// An independent color reading can recover a line obscured by map labels.
/// Conflicting recognized values are left for review, never chosen silently.
pub fn merge_stats(lines: &mut Vec<Line>, extra: Vec<Line>) {
    for line in extra {
        let Some(value) = stat(&line.text) else {
            continue;
        };
        let nearby = lines.iter().position(|l| {
            (l.y + l.height / 2. - line.y - line.height / 2.).abs()
                < l.height.min(line.height).max(8.) * 0.6
                && (l.x + l.width / 2. - line.x - line.width / 2.).abs() < 90.
        });
        if let Some(i) = nearby {
            match stat(&lines[i].text) {
                Some(old) if old != value => {
                    lines[i].text = format!("Conflicting OCR readings: {old} / {value}");
                    lines[i].source = None;
                }
                Some(_) => {}
                None => lines[i] = line,
            }
        } else {
            lines.push(line);
        }
    }
}

fn modifier_header(text: &str) -> Option<(String, String)> {
    let k = key(text);
    let words: Vec<_> = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let eater = words.iter().any(|w| w == "eater") || k.contains("ofworlds");
    let exarch = words.iter().any(|w| w == "searing" || w == "exarch");
    static TIER: OnceLock<Regex> = OnceLock::new();
    let tier = TIER
        .get_or_init(|| Regex::new(r"(?i)\b(tier|rank)\s*[:;]\s*(\d{1,2})\b").unwrap())
        .captures(text);
    if (!k.contains("modifier") && !k.contains("implicit") && !eater && !exarch && tier.is_none())
        || stat(text).is_some()
    {
        return None;
    }
    let kind = if k.contains("implicit") || eater || exarch {
        "Implicit"
    } else if k.contains("crafted") {
        "Crafted"
    } else if k.contains("fractured") {
        "Fractured"
    } else {
        "Explicit"
    };
    let mut header = kind.to_string();
    if eater {
        header.insert_str(0, "Eater of Worlds ");
    } else if exarch {
        header.insert_str(0, "Searing Exarch ");
    }
    if k.contains("prefix") {
        header.push_str(" Prefix");
    } else if k.contains("suffix") {
        header.push_str(" Suffix");
    }
    header.push_str(" Modifier");
    if let Some(c) = tier {
        let label = if c[1].eq_ignore_ascii_case("tier") {
            "Tier"
        } else {
            "Rank"
        };
        header.push_str(&format!(" ({label}: {})", &c[2]));
    }
    Some((kind.to_ascii_lowercase(), header))
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
/// Preserve the scout's catalogue anchor if a detailed reading loses a header
/// to influence ornaments. Coordinates remain relative to the source image.
pub fn anchors(lines: &[Line]) -> Vec<Line> {
    let mut sorted = lines.to_vec();
    sorted.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let Ok((i, _)) = select_item(&sorted) else {
        return Vec::new();
    };
    let base = &sorted[i];
    let mut out = Vec::new();
    if let Some(name) = sorted[..i].iter().rev().find(|l| {
        base.y - l.y < 70.
            && key(&l.text) != key(&base.text)
            && (l.x + l.width / 2. - base.x - base.width / 2.).abs() < 42.
    }) {
        out.push(name.clone());
    }
    out.push(base.clone());
    out
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
    let mut kind = "explicit".to_string();
    let mut modifier = "Explicit Modifier".to_string();
    let mut written = false;
    for line in lines.iter().skip(index + 1).filter(|l| in_column(l)) {
        if line.y - last_y > 95. {
            break;
        }
        last_y = line.y + line.height;
        let k = key(&line.text);
        if let Some((source, header)) = modifier_header(&line.text) {
            kind = source;
            modifier = header;
            written = false;
            continue;
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
            if let Some(source) = line.source.as_deref().filter(|s| *s != kind) {
                let title = match source {
                    "fractured" => "Fractured Modifier",
                    "crafted" => "Crafted Modifier",
                    _ => "Explicit Modifier",
                };
                modifier = if !written && kind == "explicit" {
                    modifier.replacen("Explicit", title.trim_end_matches(" Modifier"), 1)
                } else {
                    title.into()
                };
                kind = source.into();
                written = false;
            }
            if !written {
                text.push_str(&format!("{{ {modifier} }}\n"));
                written = true;
            }
            text.push_str(&format!("{s}\n"));
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
            source: None,
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
    #[test]
    fn advanced_armour_keeps_rolls_sources_and_hybrid_groups() {
        let text = [
            "Blight Skin",
            "Necrotic Armour",
            "Quality: +20%",
            "Evasion Rating: 821 (723-831)",
            "Energy Shield: 165 (145-167)",
            "Item Level: 86",
            "Requires Level 84, 99 Str (gem), 173 Dex, 173 Int",
            "EATER OF Wortps IMPLIGIT MODIFIER (LESSER) — Physical, Elemental, Lightning",
            "10% of Physical Damage from Hits taken as Lightning Damage",
            "Fractured Prefix Modifier Incorporeal (Tier; 1) — Defences",
            "108(101-110)% increased Evasion and Energy Shield",
            "Prefix Modifier Dragonfly's (Tier: 2) — Defences",
            "38(33-38)% increased Evasion and Energy Shield",
            "14(14-15)% increased Stun and Block Recovery",
            "Prefix Modifier Phantasm's (Tier: 1) — Defences",
            "+318(301-375) to Evasion Rating",
            "+74(73-80) to maximum Energy Shield",
            "SuFAX MapirieR OF EPHIJ (TIER; 1) — Elemental",
            "+46(46-48)% to Lightning Resistance",
            "SuFAX ModifieR OF THE POLAR BEAR (Tier; 3) — Elemental",
            "+37(36-41)% to Cold Resistance",
            "MASTER CRARTED SUFFIX MODIFIER OF CRART (RANK; 3) — Elemental",
            "+33(29-35)% to Fire Resistance",
        ];
        let mut lines: Vec<_> = text
            .iter()
            .enumerate()
            .map(|(i, s)| line(s, i as f64 * 22., (i < 2).then_some(Rarity::Rare)))
            .collect();
        lines.last_mut().unwrap().source = Some("crafted".into());
        let anchor = anchors(&lines);
        assert_eq!(anchor.len(), 2);
        assert_eq!(anchor[1].text, "Necrotic Armour");
        let d = draft(lines, "test".into()).unwrap();
        assert!(d.unknown.is_empty(), "{:?}", d.unknown);
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(i.item_level, Some(86));
        assert_eq!(i.base, "Necrotic Armour");
        let fractured = i.modifiers.iter().find(|m| m.kind == "fractured").unwrap();
        assert_eq!(fractured.value, Some(108.));
        assert_eq!(fractured.min, Some(101.));
        assert_eq!(fractured.max, Some(110.));
        assert_eq!(fractured.tier, Some(1));
        assert!(fractured.fractured_t1);
        let implicit = i.modifiers.iter().find(|m| m.kind == "implicit").unwrap();
        assert_eq!(implicit.value, Some(10.));
        assert_eq!(
            implicit.sources[0].influence.as_deref(),
            Some("eldritch-eater")
        );
        assert!(i
            .modifiers
            .iter()
            .any(|m| m.kind == "crafted" && m.value == Some(33.)));
        assert!(i.modifiers.iter().any(|m| m.value == Some(318.)));
        assert!(d
            .text
            .contains("38(33-38)% increased Evasion and Energy Shield\n14(14-15)%"));
    }
    #[test]
    fn independent_readings_recover_obscured_stats_but_reject_conflicting_values() {
        let mut lines = vec![
            line("Driftwood Wand", 0., None),
            line("NPC +318(301-375) TO EVASION RATING", 30., None),
            line("+37% to Cold Resistance", 60., None),
        ];
        merge_stats(
            &mut lines,
            vec![
                line("+318(301-375) to Evasion Rating", 30., None),
                line("+87% to Cold Resistance", 60., None),
            ],
        );
        let d = draft(lines, "test".into()).unwrap();
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert!(i.modifiers.iter().any(|m| m.value == Some(318.)));
        assert_eq!(i.modifiers.len(), 1);
        assert_eq!(d.unknown.len(), 1);
        assert!(d.unknown[0].starts_with("Conflicting OCR readings:"));
    }
    #[test]
    fn greater_eldritch_tier_does_not_match_the_eater_name() {
        assert!(stat("Searing Exarch Implicit Modifier (Greater)").is_none());
        let (kind, header) = modifier_header("Searing Exarch Implicit Modifier (Greater)").unwrap();
        assert_eq!(kind, "implicit");
        assert!(header.starts_with("Searing Exarch "));
        assert!(!header.contains("Eater"));
    }
}
