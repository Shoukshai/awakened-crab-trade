//! Shared numeric rolls and initial Trade bounds from filters/util.ts.
use crate::{
    item::{Item, Rarity},
    modifiers::Modifier,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Roll {
    pub value: f64,
    pub min: f64,
    pub max: f64,
}
impl Roll {
    pub fn of(m: &Modifier) -> Option<Self> {
        Some(Self {
            value: m.value?,
            min: m.min?,
            max: m.max?,
        })
    }
    pub fn add(&mut self, other: Self, multi: f64) {
        self.value += other.value * multi;
        self.min += other.min * multi;
        self.max += other.max * multi;
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub modifier: Modifier,
    pub display_value: Option<f64>,
    pub display_reference: String,
    pub initial_min: Option<f64>,
    pub initial_max: Option<f64>,
    pub disabled: bool,
    pub hidden: Option<String>,
}
fn precision(value: f64, decimal: bool) -> f64 {
    if !decimal || value.abs() >= 10. {
        1.
    } else if value.abs() < 2.3 {
        100.
    } else {
        10.
    }
}
pub fn round_roll(value: f64, decimal: bool) -> f64 {
    let p = precision(value, decimal);
    (value * p).trunc() / p
}
fn percent(value: f64, delta: f64, pct: f64, ceil: bool, decimal: bool) -> f64 {
    let p = precision(value, decimal);
    let value = ((value + delta * pct / 100.) + f64::EPSILON) * p;
    if ceil {
        value.ceil() / p
    } else {
        value.floor() / p
    }
}
impl Candidate {
    pub fn new(mut modifier: Modifier, item: &Item, range: f64, disabled: bool) -> Self {
        let rarity = &item.rarity;
        let roll = Roll::of(&modifier);
        let decimal = modifier.decimal;
        let mut pct = if *rarity == Rarity::Normal {
            100.
        } else {
            range
        };
        let mut display_value = modifier.value.map(|v| round_roll(v, decimal));
        let (mut initial_min, mut initial_max) = (None, None);
        if let Some(roll) = roll.filter(|_| !modifier.option) {
            let immutable_magic =
                *rarity == Rarity::Magic && (item.corrupted || item.mirrored || item.unmodifiable);
            let perfect_special = (*rarity == Rarity::Unique
                || (*rarity == Rarity::Magic && item.category.as_deref() == Some("Jewel"))
                || modifier.fractured_t1)
                && ((modifier.better == 1 && roll.value >= roll.max)
                    || (modifier.better == -1 && roll.value <= roll.min));
            if immutable_magic || perfect_special {
                pct = 0.;
            }
            let delta = if *rarity == Rarity::Unique {
                roll.max - roll.min
            } else {
                roll.value.abs()
            };
            let min = if modifier.better == 0 {
                roll.value
            } else {
                percent(roll.value, delta, -pct, false, decimal)
            };
            let max = if modifier.better == 0 {
                roll.value
            } else {
                percent(roll.value, delta, pct, true, decimal)
            };
            let min = min.max(percent(roll.min, 0., 0., false, decimal));
            let max = max.min(percent(roll.max, 0., 0., true, decimal));
            if modifier.better >= 0 {
                initial_min = Some(min);
            }
            if modifier.better <= 0 {
                initial_max = Some(max);
            }
        }
        let (display_reference, negate) = crate::modifiers::translation::display(&modifier);
        if negate {
            display_value = display_value.map(|v| -v);
            (initial_min, initial_max) = (initial_max.map(|v| -v), initial_min.map(|v| -v));
            modifier.inverted = !modifier.inverted;
        }
        if ["pseudo", "property"].contains(&modifier.kind.as_str()) {
            let number = display_value.map(|v| v.to_string()).unwrap_or_default();
            modifier.text = if number.starts_with('-') && display_reference.contains("+#") {
                display_reference.replacen("+#", &number, 1)
            } else {
                display_reference.replacen('#', &number, 1)
            };
        }
        Self {
            modifier,
            display_value,
            display_reference,
            initial_min,
            initial_max,
            disabled,
            hidden: None,
        }
    }
}
pub fn candidates(item: &Item, range: f64) -> Vec<Candidate> {
    let equipment = crate::properties::equipment(item, range);
    let pseudos = crate::pseudo::apply(item, &equipment.remaining, range);
    let mut candidates = equipment.filters;
    candidates.extend(pseudos.filters);
    candidates.extend(
        pseudos
            .remaining
            .into_iter()
            .map(|m| Candidate::new(m, item, range, true)),
    );
    // Keep consumed exact sources accessible for deliberate advanced searches.
    for modifier in &item.modifiers {
        if !candidates
            .iter()
            .any(|c| c.modifier.kind == modifier.kind && c.modifier.reference == modifier.reference)
        {
            let mut source = Candidate::new(modifier.clone(), item, range, true);
            source.hidden = Some("source_of_calculated_filter".into());
            candidates.push(source);
        }
    }
    candidates
}
