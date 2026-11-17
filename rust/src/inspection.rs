//! Local item/map inspection. Decisions belong to the player's three profiles;
//! their default values and transition rules come from upstream map-check.
use crate::{
    item::{Item, Rarity},
    modifiers::translation,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::OnceLock};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SelectedStat {
    pub matcher: String,
    pub decision: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapConfig {
    pub profile: u8,
    pub show_new_stats: bool,
    pub selected_stats: Vec<SelectedStat>,
}
impl Default for MapConfig {
    fn default() -> Self {
        #[derive(Deserialize)]
        struct Data {
            config: MapConfig,
        }
        serde_json::from_str::<Data>(include_str!("data/map-check-defaults.json"))
            .expect("upstream map-check defaults")
            .config
    }
}
#[derive(Debug, Clone)]
pub enum Edit {
    Profile(u8),
    ShowNew(bool),
    Cycle(String),
    Seen(String),
    Set(String, char),
    Clear(String),
}
impl MapConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=3).contains(&self.profile) {
            return Err("Le profil de carte doit être compris entre 1 et 3.".into());
        }
        if self.selected_stats.len() > 4096 {
            return Err("Trop de règles de cartes.".into());
        }
        let mut seen = HashSet::new();
        for stat in &self.selected_stats {
            if stat.matcher.is_empty()
                || stat.matcher.len() > 8192
                || !seen.insert(&stat.matcher)
                || stat.decision.len() != 3
                || !stat.decision.chars().all(|c| "-dwgs".contains(c))
            {
                return Err("Règle de carte invalide ou dupliquée.".into());
            }
        }
        Ok(())
    }
    pub fn decision(&self, matcher: &str) -> char {
        self.selected_stats
            .iter()
            .find(|s| s.matcher == matcher)
            .and_then(|s| {
                s.decision
                    .as_bytes()
                    .get(self.profile.saturating_sub(1) as usize)
            })
            .copied()
            .unwrap_or(b'-') as char
    }
    pub fn edit(&mut self, edit: Edit) -> Result<(), String> {
        match edit {
            Edit::Profile(profile) => {
                if !(1..=3).contains(&profile) {
                    return Err("Profil invalide.".into());
                }
                self.profile = profile;
            }
            Edit::ShowNew(show) => self.show_new_stats = show,
            other => {
                let (matcher, value) = match other {
                    Edit::Cycle(m) => {
                        let value = next_decision(self.decision(&m));
                        (m, value)
                    }
                    Edit::Seen(m) => {
                        if !self.show_new_stats {
                            return Ok(());
                        }
                        let value = if self.decision(&m) == '-' { 's' } else { '-' };
                        (m, value)
                    }
                    Edit::Set(m, value) => (m, value),
                    Edit::Clear(m) => (m, '-'),
                    _ => unreachable!(),
                };
                if !"-dwgs".contains(value) || matcher.is_empty() || matcher.len() > 8192 {
                    return Err("Décision invalide.".into());
                }
                if let Some(stat) = self
                    .selected_stats
                    .iter_mut()
                    .find(|s| s.matcher == matcher)
                {
                    stat.decision = decision_create(value, self.profile, &stat.decision);
                } else {
                    self.selected_stats.push(SelectedStat {
                        matcher,
                        decision: decision_create(value, self.profile, "---"),
                    });
                }
            }
        }
        self.validate()
    }
    pub fn outdated(&self) -> Vec<String> {
        self.selected_stats
            .iter()
            .filter(|s| {
                decision_has_color(&s.decision, self.profile)
                    && !translation::has_matcher(&s.matcher)
            })
            .map(|s| s.matcher.clone())
            .collect()
    }
}
pub fn next_decision(current: char) -> char {
    match current {
        'd' => 'w',
        'w' => 'g',
        'g' => 's',
        _ => 'd',
    }
}
pub fn decision_has_color(set: &str, profile: u8) -> bool {
    set.chars()
        .nth(profile.saturating_sub(1) as usize)
        .is_some_and(|c| c != '-' && c != 's')
}
pub fn decision_create(value: char, profile: u8, set: &str) -> String {
    let mut chars: Vec<_> = set.chars().collect();
    if (1..=3).contains(&profile) && chars.len() == 3 {
        chars[profile as usize - 1] = value;
    }
    chars.into_iter().collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogStat {
    pub matcher: String,
    pub tag: String,
}
pub fn catalog() -> &'static Vec<CatalogStat> {
    static CATALOG: OnceLock<Vec<CatalogStat>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut entries: Vec<_> = translation::area_matchers()
            .into_iter()
            .map(|(matcher, tag)| CatalogStat { matcher, tag })
            .collect();
        let order = |tag: &str| match tag {
            "ubermap_exclusive" => 1,
            "heist_exclusive" => 2,
            _ => 0,
        };
        entries.sort_by_cached_key(|s| (order(&s.tag), s.matcher.to_lowercase()));
        entries
    })
}
pub fn filtered_catalog(config: &MapConfig, search: &str, only_selected: bool) -> Vec<CatalogStat> {
    let words: Vec<_> = search
        .to_lowercase()
        .split(' ')
        .map(str::to_string)
        .collect();
    let mut out: Vec<_> = config
        .outdated()
        .into_iter()
        .map(|matcher| CatalogStat {
            matcher,
            tag: "outdated".into(),
        })
        .collect();
    out.extend(
        catalog()
            .iter()
            .filter(|s| {
                words.iter().all(|w| s.matcher.to_lowercase().contains(w))
                    && (!only_selected || matches!(config.decision(&s.matcher), 'd' | 'w' | 'g'))
            })
            .cloned(),
    );
    out
}
#[derive(Debug, Clone, Serialize)]
pub struct PreparedStat {
    pub matcher: String,
    pub roll: Option<f64>,
    pub text: String,
    pub decision: char,
    pub is_new: bool,
}
pub fn prepare(modifier: &crate::modifiers::Modifier, config: &MapConfig) -> PreparedStat {
    let (matcher, negate) = translation::display(modifier);
    let mut roll = modifier
        .value
        .map(|v| crate::calculated::round_roll(v, modifier.decimal));
    if negate {
        roll = roll.map(|v| -v);
    }
    let text = render_roll(&matcher, roll);
    let decision = config.decision(&matcher);
    PreparedStat {
        matcher,
        roll,
        text,
        decision,
        is_new: config.show_new_stats && decision == '-',
    }
}
pub fn render_roll(matcher: &str, roll: Option<f64>) -> String {
    let value = roll
        .map(|v| if v == 0. { "0".into() } else { v.to_string() })
        .unwrap_or_else(|| "#".into());
    // ItemModifierText.vue replaces every signed placeholder with the same roll.
    let mut output = String::new();
    let chars: Vec<_> = matcher.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if (chars[i] == '+' || chars[i] == '-')
            && chars.get(i + 1) == Some(&'#')
            && (i == 0 || chars[i - 1] != '#')
        {
            output.push_str(&value);
            i += 2;
        } else if chars[i] == '#' && (i == 0 || chars[i - 1] != '#') {
            output.push_str(&value);
            i += 1;
        } else {
            output.push(chars[i]);
            i += 1;
        }
    }
    output
}
pub fn is_map_like(item: &Item) -> bool {
    (item.category.as_deref() == Some("Map") && item.rarity != Rarity::Unique)
        || [
            Some("Heist Contract"),
            Some("Heist Blueprint"),
            Some("Invitation"),
            Some("Chart"),
        ]
        .contains(&item.category.as_deref())
        || item.reference == "Expedition Logbook"
        || (item.class == "Maps" && item.rarity != Rarity::Unique)
}
#[derive(Debug, Clone, Serialize)]
pub struct Dps {
    pub physical: f64,
    pub elemental: f64,
    pub total: f64,
}
pub fn item_dps(item: &Item) -> Option<Dps> {
    let aps = item.properties.aps.filter(|v| *v != 0.)?;
    let physical = (aps * item.properties.physical.unwrap_or(0.) + 0.5).floor();
    let elemental = (aps * item.properties.elemental.unwrap_or(0.) + 0.5).floor();
    Some(Dps {
        physical,
        elemental,
        total: physical + elemental,
    })
}
pub fn dust(item: &Item) -> Option<u64> {
    let value = item.unique_dust_value?;
    if value == 0. {
        return None;
    }
    let level = item.item_level? as f64;
    let corrupt_mods = if item.corrupted {
        item.raw
            .lines()
            .filter(|l| l.starts_with('{') && l.contains("Corruption Implicit Modifier"))
            .count()
    } else {
        0
    };
    let factors = (100.
        + item.influences.len() as f64 * 50.
        + item.quality.unwrap_or(0) as f64 * 2.
        + corrupt_mods as f64 * 50.)
        / 100.;
    let t = level.clamp(46., 68.) - 46.;
    let total = 5.
        * (50. + 2. * t + (3. * t / 11.).floor() + 25. * (level.clamp(68., 84.) - 68.))
        * factors;
    let result = (value * total).floor();
    (result.is_finite() && result >= 0. && result <= u64::MAX as f64).then_some(result as u64)
}
fn encode_component(text: &str) -> String {
    text.bytes()
        .map(|c| {
            if c.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&c) {
                (c as char).to_string()
            } else {
                format!("%{c:02X}")
            }
        })
        .collect()
}
#[derive(Debug, Clone, Serialize)]
pub struct Links {
    pub wiki: String,
    pub poedb: String,
    pub craft_of_exile: Option<String>,
}
pub fn links(item: &Item) -> Links {
    Links {
        wiki: format!(
            "https://www.poewiki.net/wiki/{}",
            encode_component(&item.reference)
        ),
        poedb: format!(
            "https://poedb.tw/us/{}",
            encode_component(&item.reference.replace('\'', "").replace(' ', "_"))
        ),
        craft_of_exile: (item.category.is_some() && !item.corrupted && !item.mirrored).then(|| {
            format!(
                "https://craftofexile.com/?game=poe1&eimport={}",
                encode_component(&item.raw)
            )
        }),
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub map_like: bool,
    pub title: String,
    pub unidentified: bool,
    pub maven_reminder: bool,
    pub stats: Vec<PreparedStat>,
    pub unknown: Vec<String>,
    pub outdated: Vec<String>,
    pub dps: Option<Dps>,
    pub dust: Option<u64>,
    pub links: Links,
}
pub fn report(item: &Item, config: &MapConfig) -> Report {
    let map_like = is_map_like(item);
    Report {
        map_like,
        title: if map_like {
            item.map_area
                .clone()
                .unwrap_or_else(|| item.reference.clone())
        } else {
            item.reference.clone()
        },
        unidentified: item.unidentified,
        maven_reminder: item.map_area.as_deref().is_some_and(|m| {
            [
                "Courtyard of Wasting",
                "Chambers of Impurity",
                "Theatre of Lies",
            ]
            .contains(&m)
        }),
        stats: if map_like {
            item.modifiers
                .iter()
                .filter(|m| m.kind == "explicit")
                .map(|m| prepare(m, config))
                .collect()
        } else {
            vec![]
        },
        unknown: if map_like {
            item.unknown_modifiers
                .iter()
                .filter(|m| m.kind == "explicit")
                .map(|m| m.text.clone())
                .collect()
        } else {
            vec![]
        },
        outdated: config.outdated(),
        dps: item_dps(item),
        dust: dust(item),
        links: links(item),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PHYSICAL: &str = "Monsters reflect #% of Physical Damage";
    #[test]
    fn profiles_cycle_seen_and_clear_preserve_other_profiles() {
        let mut c = MapConfig::default();
        assert_eq!(c.decision(PHYSICAL), 'd');
        c.edit(Edit::Profile(2)).unwrap();
        c.edit(Edit::Cycle(PHYSICAL.into())).unwrap();
        assert_eq!(c.decision(PHYSICAL), 'd');
        c.edit(Edit::Clear(PHYSICAL.into())).unwrap();
        assert_eq!(c.decision(PHYSICAL), '-');
        c.edit(Edit::Seen(PHYSICAL.into())).unwrap();
        assert_eq!(c.decision(PHYSICAL), '-');
        c.edit(Edit::ShowNew(true)).unwrap();
        c.edit(Edit::Seen(PHYSICAL.into())).unwrap();
        assert_eq!(c.decision(PHYSICAL), 's');
        c.edit(Edit::Seen(PHYSICAL.into())).unwrap();
        assert_eq!(c.decision(PHYSICAL), '-');
        c.edit(Edit::Profile(1)).unwrap();
        assert_eq!(c.decision(PHYSICAL), 'd');
        for expected in ['w', 'g', 's', 'd'] {
            c.edit(Edit::Cycle(PHYSICAL.into())).unwrap();
            assert_eq!(c.decision(PHYSICAL), expected);
        }
        assert_eq!(
            serde_json::from_str::<MapConfig>(&serde_json::to_string(&c).unwrap()).unwrap(),
            c
        );
    }
    #[test]
    fn rejects_invalid_profiles_decisions_and_duplicate_rules() {
        let mut c = MapConfig::default();
        assert!(c.edit(Edit::Profile(0)).is_err());
        assert!(c.edit(Edit::Profile(4)).is_err());
        assert!(c.edit(Edit::Set(PHYSICAL.into(), 'x')).is_err());
        c.selected_stats.push(c.selected_stats[0].clone());
        assert!(c.validate().is_err());
        c.selected_stats.pop();
        c.selected_stats[0].decision = "é-".into();
        assert!(c.validate().is_err());
    }
    #[test]
    fn catalog_search_keeps_outdated_colored_rules_visible() {
        let mut c = MapConfig::default();
        assert_eq!(catalog().len(), 340);
        c.edit(Edit::Set("obsolete REFLECT PHYSICAL".into(), 'w'))
            .unwrap();
        c.edit(Edit::Set("obsolete seen".into(), 's')).unwrap();

        assert!(c.outdated().contains(&"obsolete REFLECT PHYSICAL".into()));
        let filtered = filtered_catalog(&c, "OBSOLETE REFLECT PHYSICAL", true);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].tag, "outdated");
        assert!(filtered.iter().all(|s| s.tag == "outdated"));
    }
    #[test]
    fn modern_map_tier_negative_roll_unknowns_and_default_decisions() {
        let item = Item::parse(include_str!("fixtures/synthetic-map.txt")).unwrap();
        assert_eq!(item.map_tier, Some(16));
        assert_eq!(item.base, "Map");
        let mut config = MapConfig::default();
        config
            .edit(Edit::Set(
                "Rare Monsters have Physical Thorns reflecting # Physical Damage".into(),
                'd',
            ))
            .unwrap();
        let r = report(&item, &config);
        assert!(r.map_like);
        assert_eq!(r.title, "Beach");
        assert_eq!(r.stats.len(), 3);
        assert_eq!(
            r.stats.iter().map(|s| s.decision).collect::<String>(),
            "dwg"
        );
        assert_eq!(r.stats[1].roll, Some(-12.));
        assert!(r.stats[1].text.contains("-12%"));
        assert_eq!(r.unknown, vec!["Unknown future dangerous modifier 999"]);
        let magic = Item::parse(&item.raw.replace(
            "Rarity: Rare\nSynthetic Reflection\nMap (Tier 16)",
            "Rarity: Magic\nTwinned Map (Tier 16)",
        ))
        .unwrap();
        assert_eq!(magic.map_tier, Some(16));
        assert_eq!(magic.base, "Map");
        let mut unique = item;
        unique.rarity = Rarity::Unique;
        assert!(!is_map_like(&unique));
    }
    #[test]
    fn unknown_only_map_does_not_lose_its_warning() {
        let text = "Item Class: Maps\nRarity: Normal\nMap (Tier 16)\n--------\nMap Area: Beach\nItem Level: 83\n--------\n{ Prefix Modifier \"Future\" }\nCompletely unknown dangerous modifier\n";
        let item = Item::parse(text).unwrap();
        let r = report(&item, &MapConfig::default());
        assert!(r.stats.is_empty());
        assert_eq!(r.unknown, vec!["Completely unknown dangerous modifier"]);
    }
    #[test]
    fn copied_quality_dps_links_and_dust_bounds() {
        let mut item = Item::parse(include_str!("fixtures/driftwood-wand.txt")).unwrap();
        let r = report(&item, &MapConfig::default());
        assert!(!r.map_like);
        assert_eq!(r.dps.unwrap().physical, 11.);
        assert!(r
            .links
            .craft_of_exile
            .unwrap()
            .contains("Item%20Class%3A%20Wands%0A"));
        item.corrupted = true;
        assert!(links(&item).craft_of_exile.is_none());
        item.corrupted = false;
        item.mirrored = true;
        assert!(links(&item).craft_of_exile.is_none());
        item.reference = "Kaom's Heart".into();
        assert_eq!(links(&item).poedb, "https://poedb.tw/us/Kaoms_Heart");
        assert_eq!(
            links(&item).wiki,
            "https://www.poewiki.net/wiki/Kaom's%20Heart"
        );
        item.unique_dust_value = Some(1.);
        item.item_level = Some(1);
        assert_eq!(dust(&item), Some(250));
        item.item_level = Some(84);
        assert_eq!(dust(&item), Some(2500));
        item.item_level = Some(100);
        assert_eq!(dust(&item), Some(2500));
        item.unique_dust_value = Some(0.);
        assert_eq!(dust(&item), None);
    }
}
