//! Native ports of stat translations, modifier grouping/effect and aggregation.
//! Rules and data are retained from upstream; see parser/stat-translations.ts,
//! parser/advanced-mod-desc.ts and parser/modifiers.ts.
#[path = "modifiers/translation.rs"]
pub mod translation;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modifier {
    pub text: String,
    pub id: String,
    pub ids: Vec<String>,
    pub reference: String,
    pub kind: String,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub decimal: bool,
    pub legacy: bool,
    pub unscalable: bool,
    pub option: bool,
    pub inverted: bool,
    pub better: i8,
    pub source_count: usize,
    pub fractured_t1: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownModifier {
    pub text: String,
    pub kind: String,
}
fn kind_from_header(text: &str) -> &'static str {
    if text.contains("Implicit Modifier") {
        "implicit"
    } else if text.contains("Crafted") {
        "crafted"
    } else if text.contains("Fractured") {
        "fractured"
    } else {
        "explicit"
    }
}
fn suffix(text: &str) -> (&str, Option<&'static str>) {
    for kind in [
        "implicit",
        "crafted",
        "fractured",
        "enchant",
        "scourge",
        "necropolis",
        "imbued",
    ] {
        if let Some(line) = text.strip_suffix(&format!(" ({kind})")) {
            return (line, Some(kind));
        }
    }
    (text, None)
}
pub(crate) fn apply_effect(modifier: &mut Modifier, effect: f64) {
    if effect == 0. || modifier.unscalable {
        return;
    }
    let precision = if modifier.decimal { 100. } else { 1. };
    let scaled = |value: f64| {
        (((value + value * effect / 100.) + f64::EPSILON) * precision).trunc() / precision
    };
    modifier.value = modifier.value.map(scaled);
    modifier.min = modifier.min.map(scaled);
    modifier.max = modifier.max.map(scaled);
}
#[derive(Clone, Copy)]
struct Block<'a> {
    kind: &'a str,
    category: Option<&'a str>,
    effect: f64,
    fractured_t1: bool,
    report_unknown: bool,
}
fn parse_block(
    lines: &[String],
    context: Block<'_>,
    result: &mut Vec<Modifier>,
    unknown: &mut Vec<UnknownModifier>,
) {
    let Block {
        kind,
        category,
        effect,
        fractured_t1,
        report_unknown,
    } = context;
    let mut start = 0;
    let mut reminder = false;
    while start < lines.len() {
        if lines[start].starts_with('(') {
            reminder = true;
        }
        if reminder {
            if lines[start].ends_with(')') {
                reminder = false;
            }
            start += 1;
            continue;
        }
        let mut matched = false;
        // No matcher spans more physical lines than this; do not concatenate
        // unbounded tails of an unsupported clipboard section.
        let max_lines = translation::max_lines();
        let block_start = start;
        for end in block_start..lines.len().min(block_start + max_lines) {
            let text = lines[start..=end].join("\n");
            let (text, unscalable) = text
                .strip_suffix(" — Unscalable Value")
                .map(|s| (s, true))
                .unwrap_or((&text, false));
            if let Some(mut modifier) = translation::translate(text, kind, category, unscalable) {
                apply_effect(&mut modifier, effect);
                modifier.fractured_t1 = fractured_t1;
                result.push(modifier);
                start = end + 1;
                matched = true;
                break;
            }
        }
        if !matched {
            if report_unknown {
                unknown.push(UnknownModifier {
                    text: lines[start].clone(),
                    kind: kind.into(),
                });
            }
            start += 1;
        }
    }
}
/// English item text; category comes from the item's upstream catalogue entry.
#[cfg(test)]
pub fn parse(raw: &str, category: Option<&str>) -> Vec<Modifier> {
    parse_detailed(raw, category).0
}
pub fn parse_detailed(raw: &str, category: Option<&str>) -> (Vec<Modifier>, Vec<UnknownModifier>) {
    static EFFECT: OnceLock<Regex> = OnceLock::new();
    let effect_regex =
        EFFECT.get_or_init(|| Regex::new(r"([+-]?\d+(?:\.\d+)?)% Increased").unwrap());
    let mut result = Vec::new();
    let mut unknown = Vec::new();
    let mut block = Vec::new();
    let mut kind = "explicit";
    let mut base_kind = "explicit";
    let mut effect = 0.;
    let mut fractured_t1 = false;
    let mut report_unknown = false;
    for line in raw.lines().map(str::trim) {
        if line == "--------" || (line.starts_with('{') && line.ends_with('}')) {
            parse_block(
                &block,
                Block {
                    kind,
                    category,
                    effect,
                    fractured_t1,
                    report_unknown,
                },
                &mut result,
                &mut unknown,
            );
            block.clear();
            kind = if line == "--------" {
                "explicit"
            } else {
                kind_from_header(line)
            };
            base_kind = kind;
            report_unknown = line != "--------";
            fractured_t1 = kind == "fractured" && line.contains("(Tier: 1)");
            effect = effect_regex
                .captures(line)
                .and_then(|c| c[1].parse().ok())
                .unwrap_or(0.);
            continue;
        }
        let (line, explicit_kind) = suffix(line);
        let new_kind = explicit_kind.unwrap_or(base_kind);
        if new_kind != kind {
            parse_block(
                &block,
                Block {
                    kind,
                    category,
                    effect,
                    fractured_t1,
                    report_unknown,
                },
                &mut result,
                &mut unknown,
            );
            block.clear();
            kind = new_kind;
        }
        if explicit_kind.is_some() {
            report_unknown = true;
        }
        if !line.is_empty() {
            block.push(line.to_string());
        }
    }
    parse_block(
        &block,
        Block {
            kind,
            category,
            effect,
            fractured_t1,
            report_unknown,
        },
        &mut result,
        &mut unknown,
    );
    // Same stat contributed by hybrid and flat modifiers is one Trade filter.
    let mut totals: Vec<Modifier> = Vec::new();
    for modifier in result {
        if let Some(total) = totals
            .iter_mut()
            .find(|m| m.reference == modifier.reference && m.kind == modifier.kind)
        {
            if !total.option {
                let add = |a: Option<f64>, b: Option<f64>| {
                    if a.is_none() && b.is_none() {
                        None
                    } else {
                        Some(a.unwrap_or(1.) + b.unwrap_or(1.))
                    }
                };
                total.value = add(total.value, modifier.value);
                total.min = add(total.min, modifier.min);
                total.max = add(total.max, modifier.max);
            }
            total.decimal |= modifier.decimal;
            total.source_count += modifier.source_count;
            total.fractured_t1 |= modifier.fractured_t1;
            total.legacy |= modifier.legacy;
            total.text.push('\n');
            total.text.push_str(&modifier.text);
        } else {
            totals.push(modifier);
        }
    }
    (totals, unknown)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_advanced_copied_wand_keeps_implicit_roll() {
        let mods = parse("Item Class: Wands\n--------\n{ Implicit Modifier — Damage, Caster }\n8(8-12)% increased Spell Damage", None);
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].id, "implicit.stat_2974417149");
        assert_eq!(mods[0].value, Some(8.));
    }
    #[test]
    fn explicit_life_and_crafted_resistance_keep_their_kind() {
        let mods = parse(
            "+73 to maximum Life\n+32% to Fire Resistance (crafted)",
            None,
        );
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].id, "explicit.stat_3299347043");
        assert_eq!(mods[1].id, "crafted.stat_3372524247");
    }
    #[test]
    fn unknown_text_does_not_create_filters() {
        assert!(parse("unrecognized mod 99", None).is_empty());
    }
    #[test]
    fn damage_ranges_use_average_and_item_category_selects_local_id() {
        let weapon = parse("Adds 7(1-10) to 13(11-20) Fire Damage", Some("Wand"));
        let ring = parse("Adds 7(1-10) to 13(11-20) Fire Damage", Some("Ring"));
        assert_eq!(weapon[0].id, "explicit.stat_709508406");
        assert_eq!(ring[0].id, "explicit.stat_321077055");
        assert_eq!(
            (weapon[0].value, weapon[0].min, weapon[0].max),
            (Some(10.), Some(6.), Some(15.))
        );
        let per_mana = parse(
            "7 to 13 Added Attack Chaos Damage per 100 Maximum Mana",
            Some("Wand"),
        );
        assert_eq!(per_mana[0].value, Some(10.)); // 100 is literal, not a third roll.
    }
    #[test]
    fn multiline_flags_and_reminders_keep_correct_boundaries() {
        let mods = parse("Cannot be Frozen\n(Unsupported reminder\nAdds 9 to 15 Fire Damage)\n7 Lightning Damage taken per second per Power Charge if\nyour Skills have dealt a Critical Strike Recently", None);
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].id, "explicit.stat_876831634");
        assert_eq!(mods[0].value, None);
        assert_eq!(mods[1].id, "explicit.stat_1964333391");
        assert_eq!(mods[1].value, Some(7.));
    }
    #[test]
    fn advanced_kind_effect_and_unscalable_values_are_preserved() {
        let mods = parse("{ Fractured Prefix Modifier — Life — 20% Increased }\n+75(70-79) to maximum Life\n{ Master Crafted Suffix Modifier }\n+32% to Fire Resistance\n{ Eater of Worlds Implicit Modifier (Lesser) — Caster — 50% Increased }\n8(8-12)% increased Spell Damage — Unscalable Value", Some("Helmet"));
        assert_eq!(mods.len(), 3);
        assert_eq!(mods[0].kind, "fractured");
        assert_eq!(mods[0].value, Some(90.));
        assert_eq!(mods[1].id, "crafted.stat_3372524247");
        assert_eq!(mods[2].value, Some(8.));
        assert!(mods[2].unscalable);
    }
    #[test]
    fn hybrid_sources_sum_once_per_type_and_legacy_bounds_include_actual_roll() {
        let mods = parse("{ Prefix Modifier }\n+60(50-70) to maximum Life\n{ Prefix Modifier }\n+15(10-20) to maximum Life\n{ Master Crafted Prefix Modifier }\n+9 to maximum Life", None);
        assert_eq!(mods.len(), 2);
        assert_eq!(
            (mods[0].value, mods[0].min, mods[0].max),
            (Some(75.), Some(60.), Some(90.))
        );
        assert_eq!(mods[1].value, Some(9.));
        let legacy = parse("18(8-12)% increased Spell Damage", None);
        assert!(legacy[0].legacy);
        assert_eq!(legacy[0].max, Some(18.));
        let negative = parse("7(3-12)% reduced Movement Speed", None);
        assert_eq!(
            (negative[0].value, negative[0].min, negative[0].max),
            (Some(-7.), Some(-12.), Some(-3.))
        );
    }
    #[test]
    fn alternative_trade_ids_survive_resolution() {
        let mods = parse("7% chance to Avoid being Chilled during Effect", None);
        assert_eq!(
            mods[0].ids,
            vec!["explicit.stat_1619168299", "explicit.stat_1053326368"]
        );
    }
    #[test]
    fn suffix_kind_does_not_leak_into_the_following_ordinary_mod() {
        let mods = parse(
            "+32% to Fire Resistance (crafted)\n+73 to maximum Life",
            None,
        );
        assert_eq!(mods[0].kind, "crafted");
        assert_eq!(mods[1].kind, "explicit");
    }
}
