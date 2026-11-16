//! Port of parser/stat-translations.ts. The stable hashed index deliberately
//! follows upstream's duplicate/collision resolution, including advanced text.
use super::Modifier;
use regex::Regex;
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};

#[derive(Clone, Deserialize)]
struct Matcher {
    string: String,
    advanced: Option<String>,
    #[serde(default)]
    negate: bool,
    value: Option<f64>,
}
impl Matcher {
    fn matches(&self, text: &str) -> bool {
        self.string == text || self.advanced.as_deref() == Some(text)
    }
}
#[derive(Clone, Deserialize)]
struct Trade {
    ids: HashMap<String, Vec<String>>,
    #[serde(default)]
    option: bool,
    #[serde(default)]
    inverted: bool,
}
#[derive(Clone, Deserialize)]
struct Stat {
    #[serde(rename = "ref")]
    reference: String,
    better: i8,
    #[serde(default)]
    dp: bool,
    matchers: Vec<Matcher>,
    trade: Trade,
}
#[derive(Deserialize)]
struct Resolver {
    strat: String,
    #[serde(default)]
    test: Vec<Option<String>>,
    #[serde(default)]
    kind: Vec<String>,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Record {
    Group { resolve: Resolver, stats: Vec<Stat> },
    Single(Stat),
}
impl Record {
    fn stats(&self) -> &[Stat] {
        match self {
            Self::Group { stats, .. } => stats,
            Self::Single(stat) => std::slice::from_ref(stat),
        }
    }
}
struct Database {
    records: Vec<Record>,
    index: Vec<(u32, usize)>,
    ref_index: Vec<(u32, usize)>,
    max_lines: usize,
}
fn hash(text: &str) -> u32 {
    text.bytes().fold(2_166_136_261, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16_777_619)
    })
}
fn database() -> &'static Database {
    static DATA: OnceLock<Database> = OnceLock::new();
    DATA.get_or_init(|| {
        let records: Vec<Record> = include_str!("../../../renderer/public/data/en/stats.ndjson")
            .lines()
            .map(|s| serde_json::from_str(s).expect("upstream stat record"))
            .collect();
        let mut index = Vec::new();
        let mut ref_index = Vec::new();
        let mut max_lines = 1;
        for (row, record) in records.iter().enumerate() {
            for stat in record.stats() {
                ref_index.push((hash(&stat.reference), row));
                for matcher in &stat.matchers {
                    max_lines = max_lines.max(matcher.string.lines().count()).max(
                        matcher
                            .advanced
                            .as_deref()
                            .map(|s| s.lines().count())
                            .unwrap_or(1),
                    );
                    index.push((
                        hash(matcher.advanced.as_deref().unwrap_or(&matcher.string)),
                        row,
                    ));
                }
            }
        }
        index.sort_by_key(|(hash, _)| *hash);
        ref_index.sort_by_key(|(hash, _)| *hash);
        Database {
            records,
            index,
            ref_index,
            max_lines,
        }
    })
}
pub fn max_lines() -> usize {
    database().max_lines
}
/// Translation selected for the combined roll, as translateStatWithRoll does.
pub fn display(modifier: &Modifier) -> (String, bool) {
    let record = database().records.iter().find_map(|record| {
        record.stats().iter().find(|s| {
            s.reference == modifier.reference
                && s.trade
                    .ids
                    .get(&modifier.kind)
                    .is_some_and(|ids| ids.iter().any(|id| modifier.ids.contains(id)))
        })
    });
    let Some(stat) = record else {
        return (modifier.reference.clone(), false);
    };
    let matcher = if let Some(value) = modifier.value {
        stat.matchers
            .iter()
            .find(|m| m.value == Some(value))
            .or_else(|| {
                let sign = |v: f64| if v == 0. { 0. } else { v.signum() };
                let same_sign = modifier
                    .min
                    .zip(modifier.max)
                    .is_some_and(|(a, b)| sign(a) == sign(b));
                stat.matchers
                    .iter()
                    .find(|m| m.value.is_none() && m.negate == (same_sign && value < 0.))
            })
            .or_else(|| stat.matchers.iter().find(|m| m.value.is_none()))
    } else {
        stat.matchers
            .iter()
            .find(|m| m.value.is_none())
            .or_else(|| stat.matchers.first())
    };
    matcher
        .map(|m| (m.string.clone(), m.negate))
        .unwrap_or_else(|| (modifier.reference.clone(), false))
}
/// Same reference lookup and pseudo selection as assets/data.pseudoStatByRef.
pub fn pseudo(reference: &str) -> Option<Modifier> {
    let data = database();
    let index = &data.ref_index;
    let target = hash(reference);
    let (mut left, mut right) = (0isize, index.len() as isize - 1);
    while left <= right {
        let mid = (left + right) / 2;
        match index[mid as usize].0.cmp(&target) {
            std::cmp::Ordering::Less => left = mid + 1,
            std::cmp::Ordering::Greater => right = mid - 1,
            std::cmp::Ordering::Equal => {
                let record = &data.records[index[mid as usize].1];
                let stat = match record {
                    Record::Single(stat) => stat,
                    Record::Group { stats, .. } => stats
                        .iter()
                        .find(|s| s.reference == reference && s.trade.ids.contains_key("pseudo"))?,
                };
                let ids = stat.trade.ids.get("pseudo")?;
                return Some(Modifier {
                    text: stat.matchers.first()?.string.clone(),
                    id: ids.first()?.clone(),
                    ids: ids.clone(),
                    reference: stat.reference.clone(),
                    kind: "pseudo".into(),
                    value: None,
                    min: None,
                    max: None,
                    decimal: stat.dp,
                    legacy: false,
                    unscalable: false,
                    option: stat.trade.option,
                    inverted: stat.trade.inverted,
                    better: stat.better,
                    source_count: 0,
                    fractured_t1: false,
                });
            }
        }
    }
    None
}
fn lookup(text: &str) -> Option<&'static Record> {
    let data = database();
    let target = hash(text);
    // Same midpoint and equality rule as dataBinarySearch in assets/data/index.ts.
    let (mut left, mut right) = (0isize, data.index.len() as isize - 1);
    while left <= right {
        let mid = (left + right) / 2;
        let (key, row) = data.index[mid as usize];
        match key.cmp(&target) {
            std::cmp::Ordering::Less => left = mid + 1,
            std::cmp::Ordering::Greater => right = mid - 1,
            std::cmp::Ordering::Equal => {
                let record = &data.records[row];
                return record
                    .stats()
                    .iter()
                    .any(|s| s.matchers.iter().any(|m| m.matches(text)))
                    .then_some(record);
            }
        }
    }
    None
}
fn category_matches(actual: Option<&str>, expected: &str) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    match expected {
        "WEAPON" => [
            "One-Handed Axe",
            "One-Handed Mace",
            "One-Handed Sword",
            "Claw",
            "Dagger",
            "Rune Dagger",
            "Sceptre",
            "Wand",
            "Two-Handed Axe",
            "Two-Handed Mace",
            "Two-Handed Sword",
            "Staff",
            "Warstaff",
            "Fishing Rod",
            "Bow",
        ]
        .contains(&actual),
        "ARMOUR" => ["Body Armour", "Boots", "Gloves", "Helmet", "Shield"].contains(&actual),
        "HEIST_EQUIPMENT" => {
            ["Heist Tool", "Heist Brooch", "Heist Gear", "Heist Cloak"].contains(&actual)
        }
        _ => actual == expected,
    }
}
fn merge(dest: &mut Stat, source: &Stat, prefix: &str) {
    for (kind, ids) in &source.trade.ids {
        if let Some(id) = ids.first() {
            let id = format!("{prefix}{id}");
            let target = dest.trade.ids.entry(kind.clone()).or_default();
            if !target.contains(&id) {
                target.push(id);
            }
        }
    }
}
fn resolve(
    record: &Record,
    text: &str,
    kind: &str,
    category: Option<&str>,
    roll: Option<f64>,
) -> Option<Stat> {
    let Record::Group { resolve, stats } = record else {
        return Some(record.stats()[0].clone());
    };
    if resolve.strat == "select" {
        let index = resolve
            .test
            .iter()
            .position(|v| v.as_deref().is_some_and(|e| category_matches(category, e)))
            .or_else(|| resolve.test.iter().position(Option::is_none))?;
        return stats.get(index).cloned();
    }
    let on_trade: Vec<_> = stats
        .iter()
        .filter(|s| s.trade.ids.contains_key(kind))
        .collect();
    if on_trade.len() == 1 {
        return Some(on_trade[0].clone());
    }
    match resolve.strat.as_str() {
        "trivial-merge" => {
            let matching: Vec<_> = on_trade
                .into_iter()
                .filter(|s| s.matchers.iter().any(|m| m.matches(text)))
                .collect();
            let mut stat = (*matching.first()?).clone();
            for other in matching.iter().skip(1) {
                merge(&mut stat, other, "");
            }
            Some(stat)
        }
        "percent-merge" => {
            let mut percent = stats
                .get(resolve.kind.iter().position(|k| k == "percent")?)?
                .clone();
            if percent
                .matchers
                .iter()
                .any(|m| m.matches(text) && m.value == Some(100.))
            {
                let other = stats.get(resolve.kind.iter().position(|k| k == "value")?)?;
                let flag = other.matchers.len() == 1 && !other.matchers[0].string.contains('#');
                merge(
                    &mut percent,
                    other,
                    if flag {
                        "{empty_if_100}"
                    } else {
                        "{div_by_100}"
                    },
                );
                Some(percent)
            } else {
                stats
                    .iter()
                    .find(|s| s.matchers.iter().any(|m| m.matches(text)))
                    .cloned()
            }
        }
        "flag-merge" => {
            let roll = roll?;
            let mut value = stats
                .get(resolve.kind.iter().position(|k| k == "value")?)?
                .clone();
            let flag = stats.get(resolve.kind.iter().position(|k| k == "flag")?)?;
            if flag.matchers.first()?.value == Some(roll) {
                merge(&mut value, flag, "{empty}");
            }
            Some(value)
        }
        _ => None,
    }
}
#[derive(Clone)]
struct Number {
    value: f64,
    text: String,
    bounds: Option<(f64, f64)>,
    decimal: bool,
}
fn numbers(text: &str) -> (String, Vec<Number>) {
    static NUM: OnceLock<Regex> = OnceLock::new();
    static BOUNDS: OnceLock<Regex> = OnceLock::new();
    let num = NUM.get_or_init(|| Regex::new(r"([+-]?\d+(?:\.\d+)?)(?:\(([^)]+)\))?").unwrap());
    let bounds = BOUNDS
        .get_or_init(|| Regex::new(r"^([+-]?\d+(?:\.\d+)?)(?:-([+-]?\d+(?:\.\d+)?))?$").unwrap());
    let text = text.replace("()", "");
    let mut values = Vec::new();
    let mut output = String::new();
    let mut last = 0;
    for caps in num.captures_iter(&text) {
        let all = caps.get(0).unwrap();
        if all.start() > 0 && matches!(text.as_bytes()[all.start() - 1], b'0'..=b'9' | b')') {
            continue;
        }
        output.push_str(&text[last..all.start()]);
        output.push('#');
        let range = caps
            .get(2)
            .and_then(|r| bounds.captures(r.as_str()))
            .and_then(|r| {
                Some((
                    r[1].parse().ok()?,
                    r.get(2).map(|v| v.as_str()).unwrap_or(&r[1]).parse().ok()?,
                ))
            });
        if range.is_none() {
            if let Some(raw) = caps.get(2) {
                output.push('(');
                output.push_str(raw.as_str());
                if !raw.as_str().contains('-') {
                    output.push('-');
                    output.push_str(raw.as_str());
                }
                output.push(')');
            }
        }
        values.push(Number {
            value: caps[1].parse().unwrap(),
            text: caps[1].into(),
            bounds: range,
            decimal: caps[1].contains('.') || caps.get(2).is_some_and(|m| m.as_str().contains('.')),
        });
        last = all.end();
    }
    output.push_str(&text[last..]);
    (output, values)
}
const REPLACEMENTS: &[&[&[usize]]] = &[
    &[&[]],
    &[&[0], &[]],
    &[&[0, 1], &[0], &[1], &[]],
    &[&[0, 1, 2], &[1, 2], &[0, 2], &[0, 1], &[2], &[1], &[0]],
    &[
        &[0, 1, 2, 3],
        &[1, 2, 3],
        &[0, 2, 3],
        &[0, 1, 3],
        &[0, 1, 2],
        &[2, 3],
        &[1, 3],
        &[1, 2],
        &[0, 3],
        &[0, 2],
        &[0, 1],
    ],
];
/// Translate one stat string, before modifier-effect scaling and aggregation.
pub fn translate(
    text: &str,
    kind: &str,
    category: Option<&str>,
    unscalable: bool,
) -> Option<Modifier> {
    let (pattern, numbers) = numbers(text);
    let mut combinations = Vec::new();
    if let Some(replacements) = REPLACEMENTS.get(numbers.len()) {
        for literals in *replacements {
            let mut candidate = String::new();
            for (index, part) in pattern.split('#').enumerate() {
                if index > 0 {
                    candidate.push_str(if literals.contains(&(index - 1)) {
                        &numbers[index - 1].text
                    } else {
                        "#"
                    });
                }
                candidate.push_str(part);
            }
            let values = numbers
                .iter()
                .enumerate()
                .filter(|(i, _)| !literals.contains(i))
                .map(|(_, n)| n.clone())
                .collect::<Vec<_>>();
            combinations.push((candidate, values));
        }
    }
    combinations.push((text.into(), vec![]));
    for (candidate, mut values) in combinations {
        let Some(record) = lookup(&candidate) else {
            continue;
        };
        let Some(stat) = resolve(
            record,
            &candidate,
            kind,
            category,
            (values.len() == 1).then(|| values[0].value),
        ) else {
            continue;
        };
        let Some(ids) = stat.trade.ids.get(kind).filter(|ids| !ids.is_empty()) else {
            continue;
        };
        let Some(matcher) = stat.matchers.iter().find(|m| m.matches(&candidate)) else {
            continue;
        };
        let mut legacy = false;
        for number in &mut values {
            if matcher.negate {
                number.value = -number.value;
                number.bounds = number.bounds.map(|(a, b)| (-a, -b));
            }
            if stat.reference == "# uses remaining" {
                number.bounds = Some((1., number.bounds.map(|(_, b)| b).unwrap_or(number.value)));
            }
            if let Some((a, b)) = number.bounds {
                let (a, b) = (a.min(b), a.max(b));
                legacy |= number.value < a || number.value > b;
                number.bounds = Some((a.min(number.value), b.max(number.value)));
            }
        }
        if values.is_empty() {
            if let Some(value) = matcher.value.filter(|v| *v != 0.) {
                values.push(Number {
                    value,
                    text: value.to_string(),
                    bounds: Some((value, value)),
                    decimal: false,
                });
            }
        }
        let avg = |f: fn(&Number) -> f64| {
            if values.len() == 2 {
                Some((f(&values[0]) + f(&values[1])) / 2.)
            } else {
                values.first().map(f)
            }
        };
        return Some(Modifier {
            text: text.into(),
            id: ids[0].clone(),
            ids: ids.clone(),
            reference: stat.reference,
            kind: kind.into(),
            value: avg(|n| n.value),
            min: avg(|n| n.bounds.map(|(a, _)| a).unwrap_or(n.value)),
            max: avg(|n| n.bounds.map(|(_, b)| b).unwrap_or(n.value)),
            decimal: stat.dp || values.iter().any(|n| n.decimal),
            legacy,
            unscalable,
            option: stat.trade.option,
            inverted: stat.trade.inverted,
            better: stat.better,
            source_count: 1,
            fractured_t1: false,
        });
    }
    None
}
