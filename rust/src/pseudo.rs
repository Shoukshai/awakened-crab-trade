//! Rule application ported from filters/pseudo/index.ts. Exported rules retain
//! their upstream ordering, required sources, multipliers and replacement groups.
use crate::{
    calculated::{Candidate, Roll},
    item::{Item, Rarity},
    modifiers::{translation, Modifier},
};
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};
#[derive(Deserialize)]
struct Input {
    #[serde(rename = "ref")]
    reference: String,
    multiplier: Option<f64>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    keep: bool,
}
#[derive(Deserialize)]
struct Rule {
    pseudo: String,
    group: Option<String>,
    replaces: Option<String>,
    disabled: Option<bool>,
    stats: Vec<Input>,
    mutate: Option<String>,
}
fn rules() -> &'static Vec<Rule> {
    #[derive(Deserialize)]
    struct Data {
        rules: Vec<Rule>,
    }
    static RULES: OnceLock<Vec<Rule>> = OnceLock::new();
    RULES.get_or_init(|| {
        serde_json::from_str::<Data>(include_str!("data/pseudo-rules.json"))
            .expect("exported upstream rules")
            .rules
    })
}
pub struct Pseudos {
    pub filters: Vec<Candidate>,
    pub remaining: Vec<Modifier>,
}
pub fn apply(item: &Item, stats: &[Modifier], range: f64) -> Pseudos {
    if item.name == "Split Personality" {
        return Pseudos {
            filters: vec![],
            remaining: stats.to_vec(),
        };
    }
    let mut filters = Vec::<Candidate>::new();
    let mut alive = Vec::<bool>::new();
    let mut by_group = HashMap::<&str, Vec<usize>>::new();
    let mut applied = Vec::<&Rule>::new();
    for rule in rules() {
        let sources: Vec<_> = stats
            .iter()
            .filter_map(|m| {
                let info = rule.stats.iter().find(|i| i.reference == m.reference)?;
                Some((m, info, Roll::of(m)?))
            })
            .collect();
        if sources.is_empty()
            || rule
                .stats
                .iter()
                .filter(|i| i.required)
                .any(|i| !sources.iter().any(|(m, _, _)| m.reference == i.reference))
        {
            continue;
        }
        let source_count: usize = sources.iter().map(|(m, _, _)| m.source_count).sum();
        let explicit_count: usize = sources
            .iter()
            .filter(|(m, _, _)| m.kind == "explicit")
            .map(|(m, _, _)| m.source_count)
            .sum();
        if item.rarity == Rarity::Unique
            && (if item.corrupted {
                source_count < 2
            } else {
                explicit_count < 2
            })
        {
            continue;
        }
        let mut total = Roll::default();
        for (_, info, roll) in &sources {
            total.add(*roll, info.multiplier.unwrap_or(1.));
        }
        let mut modifier = translation::pseudo(&rule.pseudo).expect("upstream pseudo reference");
        modifier.value = Some(total.value);
        modifier.min = Some(total.min);
        modifier.max = Some(total.max);
        modifier.source_count = source_count;
        modifier.fractured_t1 = sources.iter().any(|(m, _, _)| m.fractured_t1);
        modifier.decimal |= sources
            .iter()
            .any(|(m, _, _)| m.reference == modifier.reference && m.decimal);
        modifier.text = format!(
            "{} · total {}",
            modifier.reference,
            crate::calculated::round_roll(total.value, modifier.decimal)
        );
        let mut filter = Candidate::new(modifier, item, range, rule.disabled.unwrap_or(true));
        if rule.mutate.as_deref() == Some("chaos_visibility") {
            if source_count == 1 && sources[0].0.kind == "crafted" {
                filter.hidden = Some("filters.hide_crafted_chaos".into());
            } else {
                filter.disabled = false;
            }
        }
        if let Some(group) = rule.replaces.as_deref().and_then(|g| by_group.get(g)) {
            for index in group {
                alive[*index] = false;
            }
        }
        let index = filters.len();
        filters.push(filter);
        alive.push(true);
        applied.push(rule);
        if let Some(group) = rule.group.as_deref() {
            by_group.entry(group).or_default().push(index);
        }
    }
    let remaining = stats
        .iter()
        .filter(|m| {
            !applied.iter().any(|r| {
                r.stats.iter().any(|s| {
                    s.reference == m.reference && (!s.keep || item.rarity != Rarity::Unique)
                })
            })
        })
        .cloned()
        .collect();
    if let Some(group) = by_group.get_mut("to_x_ele_res") {
        group.sort_by(|a, b| {
            filters[*b]
                .display_value
                .partial_cmp(&filters[*a].display_value)
                .unwrap()
        });
        let max = group.first().copied().filter(|first| {
            group.get(1).is_none_or(|second| {
                filters[*first].display_value != filters[*second].display_value
            })
        });
        for index in group {
            if Some(*index) == max {
                filters[*index].hidden = Some("filters.hide_ele_res".into());
            } else {
                alive[*index] = false;
            }
        }
    }
    if let Some(mut group) = by_group.get("to_x_attr").cloned() {
        group.sort_by(|a, b| {
            filters[*b]
                .display_value
                .partial_cmp(&filters[*a].display_value)
                .unwrap()
        });
        if group.len() == 3 {
            let to_all = by_group.get("to_all_attrs");
            if group
                .iter()
                .all(|i| filters[*i].display_value == filters[group[0]].display_value)
                && to_all.is_some()
            {
                for i in group {
                    alive[i] = false;
                }
            } else {
                if let Some(to_all) = to_all {
                    for i in to_all {
                        alive[*i] = false;
                    }
                }
                if filters[group[2]].display_value.unwrap()
                    / filters[group[0]].display_value.unwrap()
                    < 0.3
                {
                    if filters[group[1]].display_value == filters[group[2]].display_value {
                        filters[group[1]].hidden = Some("hide_attr_same_2nd_n_3rd".into());
                        filters[group[2]].hidden = Some("hide_attr_same_2nd_n_3rd".into());
                    } else {
                        filters[group[2]].hidden = Some("hide_attr_smallest_total".into());
                    }
                }
            }
        }
    }
    Pseudos {
        filters: filters
            .into_iter()
            .zip(alive)
            .filter_map(|(f, a)| a.then_some(f))
            .collect(),
        remaining,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ring(rarity: &str, mods: &str) -> Item {
        Item::parse(&format!(
            "Item Class: Rings\nRarity: {rarity}\n{}\n--------\nItem Level: 83\n--------\n{mods}",
            if rarity == "Unique" {
                "Le Heup of All\nIron Ring"
            } else {
                "Synthetic Fixture\nGold Ring"
            }
        ))
        .unwrap()
    }
    fn value(filters: &[Candidate], reference: &str) -> f64 {
        filters
            .iter()
            .find(|f| f.modifier.reference == reference)
            .unwrap()
            .modifier
            .value
            .unwrap()
    }
    #[test]
    fn resistance_weights_and_strength_life_contributions_are_not_double_counted() {
        let i=ring("Rare","{ Suffix Modifier }\n+10% to All Resistances\n{ Prefix Modifier }\n+73 to maximum Life\n{ Suffix Modifier }\n+25 to Strength\n{ Suffix Modifier }\n+20% to Fire Resistance\n{ Master Crafted Suffix Modifier }\n+15% to Cold and Chaos Resistances");
        let p = apply(&i, &i.modifiers, 10.);
        assert_eq!(value(&p.filters, "+#% total Elemental Resistance"), 65.);
        assert_eq!(value(&p.filters, "+#% total to Chaos Resistance"), 25.);
        assert_eq!(value(&p.filters, "+# total maximum Life"), 85.5);
        assert!(p.remaining.is_empty());
        let str_only = ring("Rare", "+25 to Strength");
        assert!(!apply(&str_only, &str_only.modifiers, 10.)
            .filters
            .iter()
            .any(|f| f.modifier.reference == "+# total maximum Life"));
    }
    #[test]
    fn unique_requires_multiple_explicit_sources_and_aggregated_sources_count() {
        let i=ring("Unique","{ Prefix Modifier }\n+70 to maximum Life\n{ Master Crafted Suffix Modifier }\n+20 to Strength");
        assert!(apply(&i, &i.modifiers, 10.).filters.is_empty());
        let i = ring(
            "Unique",
            "{ Prefix Modifier }\n+70 to maximum Life\n{ Prefix Modifier }\n+20 to maximum Life",
        );
        assert_eq!(i.modifiers[0].source_count, 2);
        assert_eq!(
            value(
                &apply(&i, &i.modifiers, 10.).filters,
                "+# total maximum Life"
            ),
            90.
        );
    }
    #[test]
    fn attribute_ties_replacements_and_crafted_chaos_follow_rule_order() {
        let i=ring("Rare","+10 to all Attributes\n70% increased Elemental Damage\n20% increased Lightning Damage\n+30% to Cold and Chaos Resistances (crafted)");
        let p = apply(&i, &i.modifiers, 10.);
        assert!(p
            .filters
            .iter()
            .any(|f| f.modifier.reference == "+# total to all Attributes"));
        assert!(!p
            .filters
            .iter()
            .any(|f| f.modifier.reference == "+# total to Strength"));
        assert_eq!(value(&p.filters, "#% increased Lightning Damage"), 90.);
        assert!(!p
            .filters
            .iter()
            .any(|f| f.modifier.reference == "#% increased Elemental Damage"));
        let chaos = p
            .filters
            .iter()
            .find(|f| f.modifier.reference == "+#% total to Chaos Resistance")
            .unwrap();
        assert_eq!(chaos.hidden.as_deref(), Some("filters.hide_crafted_chaos"));
        assert!(chaos.disabled);
    }
}
