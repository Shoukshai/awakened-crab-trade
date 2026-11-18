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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readings: Vec<Reading>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Reading {
    pub pass: u8,
    pub text: String,
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
                            // Some multi-line catalogue matchers contain a
                            // bare '#'. A detached ilvl is never a modifier.
                            if !s.chars().any(|c| c.is_ascii_alphabetic()) {
                                continue;
                            }
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
    static UNSCALABLE: OnceLock<Regex> = OnceLock::new();
    let unscalable =
        UNSCALABLE.get_or_init(|| Regex::new(r"(?i)\s*[—–-]\s*unscalable\s+value\s*$").unwrap());
    let has_marker = unscalable.is_match(&original);
    let original = unscalable.replace(&original, "");
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
    static BOUNDS: OnceLock<Regex> = OnceLock::new();
    let bounds = BOUNDS.get_or_init(|| {
        Regex::new(r"^\(([+-]?\d+(?:\.\d+)?)\s*[-–]\s*([+-]?\d+(?:\.\d+)?)\)$").unwrap()
    });
    for value in &values {
        if let Some(b) = value.get(2).and_then(|m| bounds.captures(m.as_str())) {
            let (v, lo, hi) = (
                value[1].parse::<f64>().ok()?,
                b[1].parse::<f64>().ok()?,
                b[2].parse::<f64>().ok()?,
            );
            if v < lo.min(hi) || v > lo.max(hi) {
                return None;
            }
        }
    }
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
        let result = if has_marker {
            format!("{result} — Unscalable Value")
        } else {
            result
        };
        if !results.contains(&result) {
            results.push(result);
        }
    }
    (results.len() == 1).then(|| results.remove(0))
}

mod readings;
pub use readings::{merge_stats, needs_retry, repair_properties};

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
    repair_properties(&mut sorted);
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
    repair_properties(&mut sorted);
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
    repair_properties(&mut lines);
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
    let mut kind = "explicit".to_string();
    let mut modifier = "Explicit Modifier".to_string();
    let mut written = false;
    for line in lines.iter().skip(index + 1).filter(|l| in_column(l)) {
        if line.y - last_y > 95. {
            break;
        }
        last_y = line.y + line.height;
        let k = key(&line.text);
        if k.is_empty() || reminder(&line.text) {
            continue;
        }
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
        if let Some((property, value)) = readings::property(&line.text) {
            // Only explicit Item Level, never "Requires Level" or a tier.
            let base = ["Armour", "Evasion Rating", "Energy Shield", "Ward"].contains(&property)
                && base_roll(&value).is_some();
            text.push_str(&format!(
                "{}{property}: {value}\n",
                if base { "Base " } else { "" }
            ));
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
    let text = check_defences(&restore_defences(&text), &mut unknown);
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
    let text = restore_defences(
        &text
            .lines()
            .map(|line| stat(line).unwrap_or_else(|| line.to_string()))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    Ok(check_defences(&text, &mut Vec::new()))
}

/// A displayed defence and its local modifiers must permit a base inside the
/// catalogue range. Contradictory pixels stay editable, never become filters.
fn check_defences(text: &str, unknown: &mut Vec<String>) -> String {
    let Ok(item) = crate::item::Item::parse(text) else {
        return text.into();
    };
    if item.rarity == Rarity::Unique || !item.unknown_modifiers.is_empty() {
        return text.into();
    }
    let mut fields = Vec::new();
    let mut refs = Vec::new();
    for (name, bounds, total, flat, incr) in [
        (
            "Armour",
            item.armour_base.ar,
            item.properties.armour,
            "+# to Armour",
            crate::properties::ARMOUR_INCR,
        ),
        (
            "Evasion Rating",
            item.armour_base.ev,
            item.properties.evasion,
            "+# to Evasion Rating",
            crate::properties::EVASION_INCR,
        ),
        (
            "Energy Shield",
            item.armour_base.es,
            item.properties.energy_shield,
            "+# to maximum Energy Shield",
            crate::properties::ES_INCR,
        ),
    ] {
        let (Some([lo, hi]), Some(total)) = (bounds, total) else {
            continue;
        };
        let Some(low) = crate::properties::from_ocr_base(&item, name, lo) else {
            continue;
        };
        let Some(high) = crate::properties::from_ocr_base(&item, name, hi) else {
            continue;
        };
        if total < low || total > high {
            fields.push(name);
            refs.push(flat);
            refs.extend_from_slice(incr);
        }
    }
    if fields.is_empty() {
        return text.into();
    }
    let warning = format!(
        "Uncertain defence values: {} disagree with the base range and local mods.",
        fields.join(", ")
    );
    unknown.push(warning.clone());
    let mut result = String::new();
    let mut kind = "explicit";
    for line in text.lines() {
        if line.starts_with('{') {
            kind = if line.contains("Implicit") {
                "implicit"
            } else if line.contains("Fractured") {
                "fractured"
            } else if line.contains("Crafted") {
                "crafted"
            } else {
                "explicit"
            };
        }
        let bad_field = fields.iter().any(|p| line.starts_with(&format!("{p}: ")));
        let bad_mod = crate::modifiers::translation::translate(
            line.trim_end_matches(" — Unscalable Value"),
            kind,
            item.category.as_deref(),
            false,
        )
        .is_some_and(|m| refs.contains(&m.reference.as_str()));
        if bad_field || bad_mod {
            let raw = format!("Uncertain: {line}");
            unknown.push(raw.clone());
            // Restore the source header after this review-only line so later
            // valid modifiers retain their kind.
            result.push_str(&format!(
                "{{ Unrecognized OCR }}\n{raw}\n{{ {} Modifier }}\n",
                match kind {
                    "implicit" => "Implicit",
                    "fractured" => "Fractured",
                    "crafted" => "Crafted",
                    _ => "Explicit",
                }
            ));
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result.push_str(&format!("{{ Unrecognized OCR }}\n{warning}\n"));
    result
}

fn reminder(text: &str) -> bool {
    let k = key(text);
    text.trim().starts_with('(')
        || k.contains("ailmentsareignited")
        || k.contains("abyssjewelscanbesocketed")
}

fn base_roll(text: &str) -> Option<[f64; 3]> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let c = RE
        .get_or_init(|| Regex::new(r"^(\d+)\s*\(\s*(\d+)\s*[-–]\s*(\d+)\s*\)$").unwrap())
        .captures(text.trim())?;
    let roll = [c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?];
    (roll[1] <= roll[0] && roll[0] <= roll[2]).then_some(roll)
}

/// Alt displays an unmodified base roll. Only rebuild the displayed defence
/// when all physical modifiers are recognized and the catalogue verifies it.
fn restore_defences(text: &str) -> String {
    let names = ["Armour", "Evasion Rating", "Energy Shield", "Ward"];
    if !text
        .lines()
        .any(|l| names.iter().any(|n| l.starts_with(&format!("Base {n}: "))))
    {
        return text.into();
    }
    let clean = text
        .lines()
        .filter(|l| {
            !(l.ends_with(" (calculated)")
                && names.iter().any(|n| l.starts_with(&format!("{n}: "))))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = clean.as_str();
    let Some(item) = crate::item::Item::parse(text).ok() else {
        return text.into();
    };
    if item
        .unknown_modifiers
        .iter()
        .any(|m| !["itemlevel", "level", "maptier", "stacksize"].contains(&key(&m.text).as_str()))
    {
        return text.into();
    }
    let mut result = text.to_string();
    for (name, bounds) in [
        ("Armour", item.armour_base.ar),
        ("Evasion Rating", item.armour_base.ev),
        ("Energy Shield", item.armour_base.es),
        ("Ward", item.armour_base.ward),
    ] {
        let Some(value) = text
            .lines()
            .find_map(|l| l.strip_prefix(&format!("Base {name}: ")))
        else {
            continue;
        };
        let Some([base, lo, hi]) = base_roll(value) else {
            continue;
        };
        if bounds != Some([lo, hi]) {
            continue;
        }
        if let Some(total) = crate::properties::from_ocr_base(&item, name, base) {
            let replacement = format!("Base {name}: {value}\n{name}: {total} (calculated)");
            result = result.replace(&format!("Base {name}: {value}"), &replacement);
        }
    }
    result
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
            readings: Vec::new(),
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
            1,
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
    #[test]
    fn detached_ilvl_is_joined_only_to_an_explicit_label_on_its_baseline() {
        let mut label = line("ITEM LEVEL:", 40., None);
        label.x = 900.;
        label.width = 90.;
        let mut value = line("84", 40., None);
        value.x = 1020.;
        value.width = 18.;
        let d = draft(
            vec![
                line("Warlock Boots", 0., Some(Rarity::Rare)),
                label.clone(),
                value.clone(),
            ],
            "test".into(),
        )
        .unwrap();
        assert_eq!(
            crate::item::Item::parse(&d.text).unwrap().item_level,
            Some(84)
        );
        assert!(!d.text.contains("Modifier }\n84"));
        assert!(stat("84").is_none());
        label.text = "Requires Level:".into();
        let d = draft(
            vec![line("Warlock Boots", 0., None), label, value],
            "test".into(),
        )
        .unwrap();
        assert_eq!(crate::item::Item::parse(&d.text).unwrap().item_level, None);
    }
    #[test]
    fn a_missing_roll_range_is_not_a_value_conflict() {
        let mut l = vec![
            line("Warlock Boots", 0., None),
            line("+41 to maximum Energy Shield", 30., None),
        ];
        merge_stats(
            &mut l,
            vec![line("41(39-49) to maximum Energy Shield", 30., None)],
            1,
        );
        let d = draft(l, "test".into()).unwrap();
        assert!(d.unknown.is_empty(), "{}", d.text);
        assert!(d.text.contains("41(39-49) to maximum Energy Shield"));
    }
    #[test]
    fn distinct_passes_resolve_the_screenshots_49_versus_41_conflict() {
        let mut l = vec![
            line("Warlock Boots", 0., None),
            line("+49 to maximum Energy Shield", 30., None),
        ];
        merge_stats(
            &mut l,
            vec![line("41(39-49) to maximum Energy Shield", 30., None)],
            1,
        );
        assert!(needs_retry(&l));
        // Repeating one reading must not manufacture a second vote.
        merge_stats(
            &mut l,
            vec![line("41(39-49) to maximum Energy Shield", 30., None)],
            1,
        );
        assert!(needs_retry(&l));
        merge_stats(
            &mut l,
            vec![line("41(39-49) to maximum Energy Shield", 30., None)],
            2,
        );
        let d = draft(l, "test".into()).unwrap();
        assert!(d.unknown.is_empty(), "{}", d.text);
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(i.modifiers[0].value, Some(41.));
        assert_eq!(i.modifiers[0].max, Some(49.));
        assert!(stat("491(39-49) to maximum Energy Shield").is_none());
    }
    #[test]
    fn alt_base_defences_are_reconstructed_after_review_and_never_used_as_totals() {
        let lines = [
            "HAVOC PACE",
            "Warlock Boots",
            "Quality: +26%",
            "Energy Shield: 80 (72-83)",
            "Item Level: 84",
            "Eater of Worlds Implicit Modifier (Lesser)",
            "4% increased Action Speed",
            "Searing Exarch Implicit Modifier (Lesser)",
            "15(15-17)% chance to Avoid Elemental Ailments",
            "Prefix Modifier Hollow",
            "Has 1 Abyssal Socket — Unscalable Value",
            "Prefix Modifier (Tier: 2)",
            "41(39-49) to maximum Energy Shield",
            "Prefix Modifier (Tier: 2)",
            "38(33-38)% increased Energy Shield",
            "14(14-15)% increased Stun and Block Recovery",
            "Prefix Modifier (Tier: 4)",
            "64(56-67)% increased Energy Shield",
            "Suffix Modifier (Tier: 1)",
            "48(46-48)% to Fire Resistance",
            "Master Crafted Suffix Modifier (Rank: 3)",
            "18(17-20)% to Fire and Cold Resistances",
            "!EMENTAL AILMENTS ARE IGNITED, SCORCHED, CHILLED, FROZEN, BRITTLE, SHOCKED, AND pp",
            "(ON2Y ABYSS/ EWE2S CAN BE SOCKETED IN AN ABYSSAL",
        ];
        let l: Vec<_> = lines
            .iter()
            .enumerate()
            .map(|(i, s)| line(s, i as f64 * 22., (i < 2).then_some(Rarity::Rare)))
            .collect();
        let d = draft(l, "test".into()).unwrap();
        assert!(d.unknown.is_empty(), "{}", d.text);
        assert!(d.text.contains("Base Energy Shield: 80 (72-83)"));
        assert!(
            d.text.contains("Energy Shield: 308 (calculated)"),
            "{}",
            d.text
        );
        let i = crate::item::Item::parse(&normalize_review(&d.text).unwrap()).unwrap();
        assert_eq!(i.properties.energy_shield, Some(308.));
        let filters = crate::properties::equipment(&i, 10.);
        let es = filters
            .filters
            .iter()
            .find(|f| f.modifier.reference == "Energy Shield: #")
            .unwrap();
        assert_eq!(es.display_value, Some(308.));
        assert!(i
            .modifiers
            .iter()
            .any(|m| m.reference == "#% increased Energy Shield" && m.value == Some(102.)));
        assert!(i
            .modifiers
            .iter()
            .any(|m| m.reference == "Has # Abyssal Sockets" && m.value == Some(1.)));
        let edited = d.text.replace(
            "41(39-49) to maximum Energy Shield",
            "Uncertain Energy Shield roll",
        );
        let revised = normalize_review(&edited).unwrap();
        assert!(!revised.contains("Energy Shield: 308 (calculated)"));
        assert_eq!(
            crate::item::Item::parse(&revised)
                .unwrap()
                .properties
                .energy_shield,
            None
        );
    }
    #[test]
    fn split_requirement_84_cannot_fill_an_empty_item_level_above_it() {
        let mut label = line("Item Level:", 30., None);
        label.x = 900.;
        label.width = 90.;
        let mut req = line("Requires Level", 60., None);
        req.x = 850.;
        req.width = 100.;
        let mut number = line("84", 60., None);
        number.x = 980.;
        number.width = 18.;
        let d = draft(
            vec![line("Warlock Boots", 0., None), label, req, number],
            "test".into(),
        )
        .unwrap();
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(i.item_level, None);
        assert!(i.modifiers.is_empty());
        assert_eq!(d.unknown, vec!["Item Level:"]);
    }
    #[test]
    fn compressed_impossible_defence_readings_never_become_trade_filters() {
        let l = [
            "Warlock Boots",
            "Quality: +26%",
            "Energy Shield: 308",
            "+4 to maximum Energy Shield",
            "102% increased Energy Shield",
            "48% to Fire Resistance",
        ];
        let d = draft(
            l.iter()
                .enumerate()
                .map(|(i, s)| line(s, i as f64 * 22., None))
                .collect(),
            "test".into(),
        )
        .unwrap();
        assert!(d
            .unknown
            .iter()
            .any(|s| s.starts_with("Uncertain defence values:")));
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(i.properties.energy_shield, None);
        assert!(!i
            .modifiers
            .iter()
            .any(|m| m.reference == "+# to maximum Energy Shield"));
        assert!(i.modifiers.iter().any(|m| m.value == Some(48.)));
    }
    #[test]
    fn contradictory_ranges_remain_uncertain_even_with_an_unbounded_reading() {
        let mut l = vec![
            line("Warlock Boots", 0., None),
            line("+41(39-49) to maximum Energy Shield", 30., None),
        ];
        merge_stats(
            &mut l,
            vec![line("+41(20-60) to maximum Energy Shield", 30., None)],
            1,
        );
        merge_stats(
            &mut l,
            vec![line("+41 to maximum Energy Shield", 30., None)],
            2,
        );
        assert!(needs_retry(&l));
        let d = draft(l, "test".into()).unwrap();
        assert!(crate::item::Item::parse(&d.text)
            .unwrap()
            .modifiers
            .is_empty());
    }
    #[test]
    fn a_placeholder_source_header_can_be_enriched_by_a_readable_header() {
        let mut l = vec![
            line("Warlock Boots", 0., None),
            line("Modifier", 30., None),
            line("4% increased Action Speed", 55., None),
        ];
        merge_stats(
            &mut l,
            vec![line(
                "Eater of Worlds Implicit Modifier (Lesser)",
                30.,
                None,
            )],
            2,
        );
        let d = draft(l, "test".into()).unwrap();
        let i = crate::item::Item::parse(&d.text).unwrap();
        assert_eq!(i.modifiers[0].kind, "implicit");
    }
}
