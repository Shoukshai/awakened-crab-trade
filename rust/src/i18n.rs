//! UI language and source-string catalogue. Game names, modifiers and Trade IDs
//! are never translated by the data/parser layers.
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU8, Ordering},
        OnceLock,
    },
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Fr,
}
impl Language {
    pub fn id(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
        }
    }
    pub fn legacy_default() -> Self {
        Self::Fr
    }
}
static CURRENT: AtomicU8 = AtomicU8::new(0);
pub fn set_language(language: Language) {
    CURRENT.store((language == Language::Fr) as u8, Ordering::Release);
}
pub fn language() -> Language {
    if CURRENT.load(Ordering::Acquire) == 1 {
        Language::Fr
    } else {
        Language::En
    }
}
pub fn tr(text: &str) -> String {
    translate(language(), text)
}
struct Pattern {
    fr: Regex,
    en: Regex,
    fr_parts: Vec<String>,
    en_parts: Vec<String>,
}
struct Catalogue {
    fr: HashMap<String, String>,
    en: HashMap<String, String>,
    french: HashSet<String>,
    english: HashSet<String>,
    patterns: Vec<Pattern>,
}
fn parts(template: &str) -> Vec<String> {
    static PLACEHOLDER: OnceLock<Regex> = OnceLock::new();
    PLACEHOLDER
        .get_or_init(|| Regex::new(r"\{[^{}]*\}").unwrap())
        .split(template)
        .map(str::to_owned)
        .collect()
}
fn pattern(parts: &[String]) -> Regex {
    Regex::new(&format!(
        "(?s)^{}$",
        parts
            .iter()
            .map(|s| regex::escape(s))
            .collect::<Vec<_>>()
            .join("(.*?)")
    ))
    .unwrap()
}
fn catalogue() -> &'static Catalogue {
    static DATA: OnceLock<Catalogue> = OnceLock::new();
    DATA.get_or_init(|| {
        let fr: HashMap<String, String> =
            serde_json::from_str(include_str!("i18n/en.json")).expect("UI translations");
        let mut en = HashMap::new();
        let mut pairs: Vec<_> = fr.iter().collect();
        pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.0.cmp(b.0)));
        let mut patterns = Vec::new();
        for (source, target) in pairs {
            en.insert(target.clone(), source.clone());
            let a = parts(source);
            let b = parts(target);
            assert_eq!(a.len(), b.len(), "Translation placeholder count: {source}");
            if source != target && a.len() > 1 {
                patterns.push(Pattern {
                    fr: pattern(&a),
                    en: pattern(&b),
                    fr_parts: a,
                    en_parts: b,
                });
            }
        }
        Catalogue {
            french: fr.keys().cloned().collect(),
            english: fr.values().cloned().collect(),
            fr,
            en,
            patterns,
        }
    })
}
pub fn translate(language: Language, text: &str) -> String {
    translate_inner(language, text, 0)
}
fn translate_inner(language: Language, text: &str, depth: u8) -> String {
    if text.is_empty() || depth > 5 {
        return text.to_owned();
    }
    let data = catalogue();
    let (already, exact) = match language {
        Language::En => (&data.english, &data.fr),
        Language::Fr => (&data.french, &data.en),
    };
    if already.contains(text) {
        return text.to_owned();
    }
    if let Some(target) = exact.get(text) {
        return target.clone();
    }
    // Already-localized values are fixed points, including formatted messages.
    if data.patterns.iter().any(|p| match language {
        Language::En => p.en.is_match(text),
        Language::Fr => p.fr.is_match(text),
    }) {
        return text.to_owned();
    }
    for p in &data.patterns {
        let (source, target) = match language {
            Language::En => (&p.fr, &p.en_parts),
            Language::Fr => (&p.en, &p.fr_parts),
        };
        if let Some(values) = source.captures(text) {
            let mut out = target[0].clone();
            for (i, suffix) in target.iter().enumerate().skip(1) {
                out.push_str(&translate_inner(language, &values[i], depth + 1));
                out.push_str(suffix);
            }
            return out;
        }
    }
    // Composite metadata and multiline hints retain their separators and values.
    for separator in ["\n", " · "] {
        if text.contains(separator) {
            return text
                .split(separator)
                .map(|s| translate_inner(language, s, depth + 1))
                .collect::<Vec<_>>()
                .join(separator);
        }
    }
    text.to_owned()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_languages_are_idempotent_and_keep_game_terms() {
        for language in [Language::En, Language::Fr] {
            for text in catalogue().fr.keys().chain(catalogue().fr.values()) {
                let translated = translate(language, text);
                assert_eq!(translate(language, &translated), translated, "{text}");
            }
            for text in [
                "Siren's Tentacle",
                "27% increased Effect of Cold Ailments",
                "implicit.stat_1793818220",
                "Chaos Orb",
            ] {
                assert_eq!(translate(language, text), text);
            }
        }
    }
    #[test]
    fn formatted_metadata_errors_and_reverse_switching_keep_values() {
        for (fr, en) in [
            ("2 résultats", "2 results"),
            (
                "Gemme · niveau 20 · qualité 20% · Corrompu",
                "Gem · level 20 · quality 20% · Corrupted",
            ),
            ("Viscera · Jambe", "Viscera · Leg"),
            ("Autres stats · 6", "Other stats · 6"),
            (
                "Min doit être inférieur ou égal à Max.",
                "Min must be less than or equal to Max.",
            ),
        ] {
            assert_eq!(translate(Language::En, fr), en);
            assert_eq!(translate(Language::Fr, en), fr);
        }
    }
}
