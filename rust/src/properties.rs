//! Equipment-property and quality calculations from parser/calc-q20.ts and
//! filters/pseudo/item-property.ts. Trade normalizes physical damage/defences.
use crate::{
    calculated::{Candidate, Roll},
    item::Item,
    modifiers::Modifier,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ArmourBase {
    pub ar: Option<[f64; 2]>,
    pub ev: Option<[f64; 2]>,
    pub es: Option<[f64; 2]>,
    pub ward: Option<[f64; 2]>,
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Properties {
    pub armour: Option<f64>,
    pub evasion: Option<f64>,
    pub energy_shield: Option<f64>,
    pub ward: Option<f64>,
    pub block: Option<f64>,
    pub physical: Option<f64>,
    pub elemental: Option<f64>,
    pub aps: Option<f64>,
    pub crit: Option<f64>,
    pub memory_strands: Option<f64>,
}
impl Properties {
    pub fn parse(text: &str) -> Self {
        let field = |prefix: &str| {
            text.lines()
                .find_map(|l| l.trim().strip_prefix(prefix))
                .map(str::trim)
        };
        let prefix_number = |s: &str| {
            s.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+')
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|v| v.is_finite())
        };
        let number = |prefix: &str| field(prefix).and_then(prefix_number);
        let damage = |s: &str| {
            let values = s.split('-').filter_map(prefix_number).collect::<Vec<_>>();
            match values.as_slice() {
                [a, b] => Some((a + b) / 2.),
                [a] => Some(*a),
                _ => None,
            }
        };
        Self {
            armour: number("Armour: "),
            evasion: number("Evasion Rating: "),
            energy_shield: number("Energy Shield: "),
            ward: number("Ward: "),
            block: number("Chance to Block: "),
            physical: field("Physical Damage: ").and_then(damage),
            elemental: field("Elemental Damage: ").and_then(|s| {
                s.split(", ")
                    .map(damage)
                    .collect::<Option<Vec<_>>>()
                    .map(|v| v.iter().sum())
            }),
            aps: number("Attacks per Second: "),
            crit: number("Critical Strike Chance: "),
            memory_strands: number("Memory Strands: "),
        }
    }
}
pub fn is_armour(category: Option<&str>) -> bool {
    category.is_some_and(|c| ["Body Armour", "Helmet", "Gloves", "Boots", "Shield"].contains(&c))
}
pub fn is_weapon(category: Option<&str>) -> bool {
    category.is_some_and(|c| {
        [
            "One-Handed Axe",
            "Two-Handed Axe",
            "One-Handed Mace",
            "Two-Handed Mace",
            "One-Handed Sword",
            "Two-Handed Sword",
            "Claw",
            "Dagger",
            "Rune Dagger",
            "Staff",
            "Warstaff",
            "Wand",
            "Sceptre",
            "Bow",
            "Fishing Rod",
        ]
        .contains(&c)
    })
}
pub const ARMOUR_INCR: &[&str] = &[
    "#% increased Armour",
    "#% increased Armour and Energy Shield",
    "#% increased Armour and Evasion",
    "#% increased Armour, Evasion and Energy Shield",
];
pub const EVASION_INCR: &[&str] = &[
    "#% increased Evasion Rating",
    "#% increased Armour and Evasion",
    "#% increased Evasion and Energy Shield",
    "#% increased Armour, Evasion and Energy Shield",
];
pub const ES_INCR: &[&str] = &[
    "#% increased Energy Shield",
    "#% increased Armour and Energy Shield",
    "#% increased Evasion and Energy Shield",
    "#% increased Armour, Evasion and Energy Shield",
];
const QUALITY_ENCHANTS: &[&str] = &[
    "Quality does not increase Defences",
    "Quality does not increase Physical Damage",
];
fn totals(item: &Item, flat: &[&str], incr: &[&str]) -> (Roll, Roll) {
    let (mut f, mut i) = (Roll::default(), Roll::default());
    for m in &item.modifiers {
        if let Some(roll) = Roll::of(m) {
            if flat.contains(&m.reference.as_str()) {
                f.add(roll, 1.);
            } else if incr.contains(&m.reference.as_str()) {
                i.add(roll, 1.);
            }
        }
    }
    (f, i)
}
pub(crate) fn quality(item: &Item) -> (f64, f64) {
    if item
        .modifiers
        .iter()
        .any(|m| QUALITY_ENCHANTS.contains(&m.reference.as_str()))
    {
        (0., 0.)
    } else {
        let q = item.quality.unwrap_or(0) as f64;
        (q, q.max(20.))
    }
}
fn inc(base: f64, incr: f64, quality: f64) -> f64 {
    base * (1. + incr / 100.) * (1. + quality / 100.)
}
fn base(total: f64, incr: f64, quality: f64) -> f64 {
    total / (1. + quality / 100.) / (1. + incr / 100.)
}
fn q20(total: f64, flat: &[&str], incr: &[&str], bounds: Option<[f64; 2]>, item: &Item) -> Roll {
    let (flat, incr) = totals(item, flat, incr);
    let (q, q20) = quality(item);
    let base = base(total, incr.value, q) - flat.value;
    if base.is_nan() {
        return Roll::default();
    }
    let [low, high] = bounds.unwrap_or([base, base]);
    Roll {
        value: inc(base + flat.value, incr.value, q20),
        min: inc(low + flat.min, incr.min, q20),
        max: inc(high + flat.max, incr.max, q20),
    }
}
fn bounds(total: f64, flat: &[&str], incr: &[&str], item: &Item) -> Roll {
    let (flat, incr) = totals(item, flat, incr);
    let base = base(total, incr.value, 0.) - flat.value;
    Roll {
        value: inc(base + flat.value, incr.value, 0.),
        min: inc(base + flat.min, incr.min, 0.),
        max: inc(base + flat.max, incr.max, 0.),
    }
}
fn product(a: Roll, b: Roll) -> Roll {
    Roll {
        value: a.value * b.value,
        min: a.min * b.min,
        max: a.max * b.max,
    }
}
fn property(
    reference: &str,
    id: &str,
    roll: Roll,
    item: &Item,
    range: f64,
    decimal: bool,
    disabled: bool,
) -> Option<Candidate> {
    if ![roll.value, roll.min, roll.max]
        .into_iter()
        .all(f64::is_finite)
    {
        return None;
    }
    let modifier = Modifier {
        text: format!(
            "{reference} · {}",
            crate::calculated::round_roll(roll.value, decimal)
        ),
        reference: reference.into(),
        id: id.into(),
        ids: vec![id.into()],
        kind: "property".into(),
        value: Some(roll.value),
        min: Some(roll.min),
        max: Some(roll.max),
        decimal,
        legacy: false,
        unscalable: false,
        option: false,
        inverted: false,
        better: 1,
        source_count: 0,
        fractured_t1: false,
        tier: None,
        sources: vec![],
    };
    Some(Candidate::new(modifier, item, range, disabled))
}
pub struct Equipment {
    pub filters: Vec<Candidate>,
    pub remaining: Vec<Modifier>,
}
pub fn equipment(item: &Item, range: f64) -> Equipment {
    let mut filters = Vec::<Candidate>::new();
    let mut consumed = Vec::<&str>::new();
    let p = &item.properties;
    let mut push = |reference: &str,
                    id: &str,
                    roll: Roll,
                    decimal: bool,
                    disabled: bool,
                    hidden: Option<&str>| {
        if let Some(mut candidate) = property(reference, id, roll, item, range, decimal, disabled) {
            candidate.hidden = hidden.map(str::to_string);
            filters.push(candidate);
        }
    };
    if is_armour(item.category.as_deref()) {
        let single = [p.armour, p.evasion, p.energy_shield, p.ward]
            .into_iter()
            .flatten()
            .count()
            == 1;
        let attrs = [
            (
                p.armour,
                "Armour: #",
                "item.armour",
                "+# to Armour",
                ARMOUR_INCR,
                item.armour_base.ar,
            ),
            (
                p.evasion,
                "Evasion Rating: #",
                "item.evasion_rating",
                "+# to Evasion Rating",
                EVASION_INCR,
                item.armour_base.ev,
            ),
            (
                p.energy_shield,
                "Energy Shield: #",
                "item.energy_shield",
                "+# to maximum Energy Shield",
                ES_INCR,
                item.armour_base.es,
            ),
            (
                p.ward,
                "Ward: #",
                "item.ward",
                "+# to Ward",
                &["#% increased Ward"][..],
                item.armour_base.ward,
            ),
        ];
        for (value, reference, id, flat, incr, base) in attrs {
            if let Some(value) = value.filter(|v| *v != 0.) {
                push(
                    reference,
                    id,
                    q20(value, &[flat], incr, base, item),
                    false,
                    !single,
                    None,
                );
            }
            consumed.push(flat);
            consumed.extend_from_slice(incr);
        }
        if let Some(block) = p.block.filter(|v| *v != 0.) {
            push(
                "Block: #%",
                "item.block",
                bounds(block, &["+#% Chance to Block"], &[], item),
                false,
                true,
                None,
            );
        }
        consumed.push("+#% Chance to Block");
        if [p.armour, p.evasion, p.energy_shield, p.ward, p.block]
            .into_iter()
            .flatten()
            .all(|v| v == 0.)
        {
            consumed.clear();
        }
    }
    if is_weapon(item.category.as_deref()) {
        let as_roll = bounds(
            p.aps.unwrap_or(0.),
            &[],
            &["#% increased Attack Speed"],
            item,
        );
        let phys = q20(
            p.physical.unwrap_or(0.),
            &["Adds # to # Physical Damage"],
            &["#% increased Physical Damage"],
            None,
            item,
        );
        let pdps = product(phys, as_roll);
        let ele = bounds(
            p.elemental.unwrap_or(0.),
            &[
                "Adds # to # Lightning Damage",
                "Adds # to # Cold Damage",
                "Adds # to # Fire Damage",
            ],
            &[],
            item,
        );
        let edps = product(ele, as_roll);
        let mut dps = pdps;
        dps.add(edps, 1.);
        if p.elemental.is_some_and(|v| v != 0.) {
            if p.physical.is_some_and(|v| v != 0.) {
                push("Total DPS: #", "item.total_dps", dps, false, false, None);
            }
            let small = edps.value / dps.value < 0.67;
            push(
                "Elemental DPS: #",
                "item.elemental_dps",
                edps,
                false,
                small,
                small.then_some("filters.hide_ele_dps"),
            );
        }
        if p.physical.is_some_and(|v| v != 0.) {
            let important = item.category.as_deref().is_some_and(|c| {
                [
                    "One-Handed Axe",
                    "Two-Handed Axe",
                    "One-Handed Sword",
                    "Two-Handed Sword",
                    "Bow",
                    "Warstaff",
                ]
                .contains(&c)
            });
            let small = pdps.value / dps.value < 0.67;
            push(
                "Physical DPS: #",
                "item.physical_dps",
                pdps,
                false,
                !important || small,
                small.then_some("filters.hide_phys_dps"),
            );
        }
        push(
            "Attacks per Second: #",
            "item.aps",
            as_roll,
            true,
            true,
            None,
        );
        if let Some(crit) = p.crit.filter(|v| *v != 0.) {
            push(
                "Critical Strike Chance: #%",
                "item.crit",
                bounds(crit, &[], &["#% increased Critical Strike Chance"], item),
                true,
                true,
                None,
            );
        }
        if [p.aps, p.crit, p.elemental, p.physical]
            .into_iter()
            .flatten()
            .any(|v| v != 0.)
        {
            consumed.extend([
                "Adds # to # Physical Damage",
                "#% increased Physical Damage",
                "#% increased Attack Speed",
                "#% increased Critical Strike Chance",
                "Adds # to # Lightning Damage",
                "Adds # to # Cold Damage",
                "Adds # to # Fire Damage",
            ]);
        }
    }
    // Percentile is derived from the first available base defence in upstream order.
    for (value, base_bounds, flat, incr) in [
        (p.armour, item.armour_base.ar, "+# to Armour", ARMOUR_INCR),
        (
            p.evasion,
            item.armour_base.ev,
            "+# to Evasion Rating",
            EVASION_INCR,
        ),
        (
            p.energy_shield,
            item.armour_base.es,
            "+# to maximum Energy Shield",
            ES_INCR,
        ),
        (
            p.ward,
            item.armour_base.ward,
            "+# to Ward",
            &["#% increased Ward"][..],
        ),
    ] {
        if let (Some(value), Some([min, max])) = (value.filter(|v| *v != 0.), base_bounds) {
            let (flat, incr) = totals(item, &[flat], incr);
            let (q, _) = quality(item);
            let base = base(value, incr.value, q) - flat.value;
            let percentile = ((base - min) / (max - min) * 100. + 0.5)
                .floor()
                .clamp(0., 100.);
            push(
                "Base Percentile: #%",
                "item.base_percentile",
                Roll {
                    value: percentile,
                    min: 0.,
                    max: 100.,
                },
                false,
                percentile < 50.,
                None,
            );
            break;
        }
    }
    if let Some(value) = p.memory_strands {
        push(
            "Memory Strands: #",
            "item.memory_strands",
            Roll {
                value,
                min: 0.,
                max: 100.,
            },
            false,
            true,
            Some("hide_memory_strands"),
        );
    }
    Equipment {
        filters,
        remaining: item
            .modifiers
            .iter()
            .filter(|m| !consumed.contains(&m.reference.as_str()))
            .cloned()
            .collect(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_wand_uses_trade_quality_normalization_and_not_spell_damage_for_dps() {
        let i = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        let e = equipment(&i, 10.);
        let pdps = e
            .filters
            .iter()
            .find(|f| f.modifier.id == "item.physical_dps")
            .unwrap();
        assert!((pdps.modifier.value.unwrap() - 12.6).abs() < 1e-9);
        assert_eq!(pdps.initial_min, Some(12.));
        assert!(pdps.disabled); // caster wand: upstream does not suggest pDPS.
        assert_eq!(e.remaining[0].reference, "#% increased Spell Damage");
    }
    #[test]
    fn armour_base_range_and_percentile_survive_quality_normalization() {
        let i=Item::parse("Item Class: Body Armours\nRarity: Normal\nPlate Vest\n--------\nArmour: 23\nQuality: +0%\n--------\nItem Level: 1").unwrap();
        let e = equipment(&i, 10.);
        let ar = e
            .filters
            .iter()
            .find(|f| f.modifier.id == "item.armour")
            .unwrap();
        assert!((ar.modifier.value.unwrap() - 27.6).abs() < 1e-9);
        assert_eq!(ar.initial_min, Some(22.));
        let base = e
            .filters
            .iter()
            .find(|f| f.modifier.id == "item.base_percentile")
            .unwrap();
        assert_eq!(base.modifier.value, Some(50.));
    }
    #[test]
    fn quality_changing_enchant_and_higher_quality_are_respected() {
        let mut i = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        i.quality = Some(30);
        let p = equipment(&i, 10.)
            .filters
            .into_iter()
            .find(|f| f.modifier.id == "item.physical_dps")
            .unwrap();
        assert!((p.modifier.value.unwrap() - 10.5).abs() < 1e-9); // already quality 30, no extra quality.
        i.quality = Some(0);
        i.modifiers.extend(crate::modifiers::parse(
            "Quality does not increase Physical Damage (enchant)",
            i.category.as_deref(),
        ));
        let p = equipment(&i, 10.)
            .filters
            .into_iter()
            .find(|f| f.modifier.id == "item.physical_dps")
            .unwrap();
        assert!((p.modifier.value.unwrap() - 10.5).abs() < 1e-9);
    }
}
