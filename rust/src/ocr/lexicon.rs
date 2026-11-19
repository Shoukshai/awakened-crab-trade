//! Conservative local word repair. The dictionary never supplies a number.
use super::{key, numbers, stat_key};
use regex::Regex;
use std::{collections::HashMap, sync::OnceLock};

/// Banded edit distance; never allocate a matrix or search unbounded text.
pub(super) fn distance(a: &str, b: &str, limit: usize) -> Option<usize> {
    if a.len().abs_diff(b.len()) > limit {
        return None;
    }
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<_> = (0..=b.len()).collect();
    let mut next = vec![limit + 1; b.len() + 1];
    for (i, x) in a.iter().enumerate() {
        next.fill(limit + 1);
        next[0] = i + 1;
        let lo = (i + 1).saturating_sub(limit).max(1);
        let hi = (i + 1 + limit).min(b.len());
        for j in lo..=hi {
            next[j] = (prev[j] + 1)
                .min(next[j - 1] + 1)
                .min(prev[j - 1] + usize::from(*x != b[j - 1]));
        }
        if next.iter().copied().min().unwrap_or(limit + 1) > limit {
            return None;
        }
        std::mem::swap(&mut prev, &mut next);
    }
    (prev[b.len()] <= limit).then_some(prev[b.len()])
}
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}
fn compatible(observed: &str, candidate: &str) -> bool {
    let observed = words(observed);
    let candidate = words(candidate);
    // Never turn a clearly observed element, polarity or restriction into another.
    const PROTECTED: &[&str] = &[
        "increased",
        "reduced",
        "more",
        "less",
        "maximum",
        "minimum",
        "fire",
        "cold",
        "lightning",
        "chaos",
        "physical",
        "attack",
        "attacks",
        "spell",
        "not",
        "cannot",
        "vaal",
        "life",
        "mana",
        "energy",
        "shield",
        "ward",
        "armour",
        "evasion",
        "resistance",
        "resistances",
    ];
    PROTECTED
        .iter()
        .all(|w| !observed.iter().any(|s| s == w) || candidate.iter().any(|s| s == w))
}
fn rolls() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[+-]?\d+(?:\.\d+)?(?:\([+-]?\d+(?:\.\d+)?\s*[-–]\s*[+-]?\d+(?:\.\d+)?\))?")
            .unwrap()
    })
}
fn render(template: &str, observed: &str) -> Option<String> {
    static TOKENS: OnceLock<Regex> = OnceLock::new();
    let tokens = TOKENS.get_or_init(|| Regex::new(r"[+-]?#|[+-]?\d+(?:\.\d+)?").unwrap());
    let spans: Vec<_> = rolls().find_iter(observed).collect();
    let values: Vec<_> = spans.iter().map(|m| m.as_str()).collect();
    let parts: Vec<_> = tokens.find_iter(template).collect();
    if parts.len() != values.len() || observed.matches('%').count() != template.matches('%').count()
    {
        return None;
    }
    if parts.iter().zip(&spans).any(|(p, v)| {
        template[p.end()..].trim_start().starts_with('%')
            != observed[v.end()..].trim_start().starts_with('%')
    }) {
        return None;
    }
    let (mut result, mut offset) = (String::new(), 0);
    for (part, value) in parts.into_iter().zip(&values) {
        if !part.as_str().contains('#') && part.as_str() != *value {
            return None;
        }
        result.push_str(&template[offset..part.start()]);
        result.push_str(value);
        offset = part.end();
    }
    result.push_str(&template[offset..]);
    let after: Vec<_> = rolls().find_iter(&result).map(|m| m.as_str()).collect();
    (values == after).then_some(result)
}
pub(super) fn stat(observed: &str, matchers: &HashMap<String, Vec<String>>) -> Option<String> {
    let normalized = observed.replace(['−', '–'], "-");
    let observed = normalized.as_str();
    if observed.len() > 512 || observed.chars().filter(char::is_ascii_alphabetic).count() < 12 {
        return None;
    }
    static RANGE: OnceLock<Regex> = OnceLock::new();
    let range = RANGE
        .get_or_init(|| Regex::new(r"\([+-]?\d+(?:\.\d+)?\s*[-–]\s*[+-]?\d+(?:\.\d+)?\)").unwrap());
    static SUFFIX: OnceLock<Regex> = OnceLock::new();
    let suffix =
        SUFFIX.get_or_init(|| Regex::new(r"(?i)\s*[—–-]\s*unscalable\s+value\s*$").unwrap());
    let marker = suffix
        .find(observed)
        .map(|m| m.as_str())
        .unwrap_or_default();
    let text = suffix.replace(observed, "");
    let query = stat_key(&range.replace_all(&text, ""));
    let edits = (query.len() / 10).clamp(1, 3);
    let mut candidates: Vec<(usize, String, String)> = Vec::new();
    for (shape, templates) in matchers {
        if shape.matches('#').count() != query.matches('#').count() {
            continue;
        }
        let Some(d) = distance(&query, shape, edits + 2) else {
            continue;
        };
        let mut results = Vec::new();
        for template in templates {
            if !compatible(&text, template) {
                continue;
            }
            if let Some(value) = render(template, &text) {
                if super::exact_stat(&value).is_some()
                    && !results
                        .iter()
                        .any(|s: &String| s.eq_ignore_ascii_case(&value))
                {
                    results.push(value);
                }
            }
        }
        // Several literal/option translations must not be resolved by iteration order.
        if results.len() == 1 {
            candidates.push((d, shape.clone(), results.remove(0)));
        }
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let best = candidates.first()?;
    if best.0 > edits || candidates.get(1).is_some_and(|next| next.0 < best.0 + 2) {
        return None;
    }
    let result = format!("{}{marker}", best.2);
    if numbers()
        .find_iter(observed)
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        != numbers()
            .find_iter(&result)
            .map(|m| m.as_str())
            .collect::<Vec<_>>()
    {
        return None;
    }
    eprintln!(
        "ocr catalogue wording observed={observed:?} matched={result:?} edits={}",
        best.0
    );
    Some(result)
}
pub(super) fn name<'a>(
    observed: &str,
    candidates: impl Iterator<Item = &'a str>,
) -> Option<String> {
    if observed.chars().any(|c| c.is_ascii_digit()) || observed.len() > 80 {
        return None;
    }
    let query = key(observed);
    if query.len() < 8 {
        return None;
    }
    let limit = if query.len() >= 20 { 2 } else { 1 };
    let mut ranked = Vec::new();
    for name in candidates {
        if !compatible(observed, name) {
            continue;
        }
        let normalized = key(name);
        if let Some(d) = distance(&query, &normalized, limit + 2) {
            ranked.push((d, normalized));
        }
    }
    ranked.sort_unstable();
    let best = ranked.first()?;
    if best.0 > limit || ranked.get(1).is_some_and(|other| other.0 < best.0 + 2) {
        return None;
    }
    Some(best.1.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dictionary_keeps_signed_decimal_and_range_strings_exactly() {
        assert_eq!(
            render(
                "+#% to Lightning Resistance",
                "+25(24-29)% to Lightnlng Resistance"
            )
            .as_deref(),
            Some("+25(24-29)% to Lightning Resistance")
        );
        assert_eq!(
            render(
                "Adds # to # Cold Damage",
                "Adds 1.5(1-2) to 4.5(4-5) Cold Damagc"
            )
            .as_deref(),
            Some("Adds 1.5(1-2) to 4.5(4-5) Cold Damage")
        );
        assert_eq!(
            render(
                "Non-Channelling Skills have +# to Total Mana Cost",
                "Non-Channelling Skills have -7(-7--6) to Total Mana Cosl"
            )
            .as_deref(),
            Some("Non-Channelling Skills have -7(-7--6) to Total Mana Cost")
        );
        assert!(render("Grants Level 20 Skill", "Grants Level 19 Skill").is_none());
        assert!(render("#% increased Movement Speed", "0 increased Movement Speed").is_none());
        assert!(render("Adds #% to # Cold Damage", "Adds 1 to 2% Cold Damage").is_none());
    }
    #[test]
    fn words_and_ambiguous_names_are_not_forced() {
        assert!(!compatible("increased Fire Damage", "reduced Cold Damage"));
        assert!(name("Warlock Bots", ["warlockboots", "warlockbolts"].into_iter()).is_none());
        assert_eq!(
            name(
                "Warlock Bots",
                ["warlockboots", "sorcererboots"].into_iter()
            )
            .as_deref(),
            Some("warlockboots")
        );
        assert!(name("Item Level 85", ["itemlevel86"].into_iter()).is_none());
        assert_eq!(distance("abc", "adc", 1), Some(1));
        assert_eq!(distance("abc", "axxx", 1), None);
    }
}
