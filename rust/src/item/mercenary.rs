//! Mercenary clipboard sections and filter models from Parser.ts and mercenary.ts.
#[path = "mercenary/query.rs"]
mod query;
use crate::modifiers::{
    translation::{self, MercenaryGem},
    UnknownModifier,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildSkill {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum BuildRecord {
    Alias(String),
    Data { skills: Vec<BuildSkill> },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parsed {
    pub build: String,
    pub level: Option<u32>,
    pub skills: Vec<Vec<String>>,
    pub unknown: Vec<UnknownModifier>,
}
fn build_skills(name: &str) -> Option<Vec<BuildSkill>> {
    let entries = super::catalogue();
    let mut entry = entries
        .iter()
        .find(|e| e.namespace == "MERCENARY_BUILD" && e.name == name)?;
    if let Some(BuildRecord::Alias(alias)) = &entry.mercenary_build {
        entry = entries
            .iter()
            .find(|e| e.namespace == "MERCENARY_BUILD" && e.reference == *alias)?;
    }
    match entry.mercenary_build.as_ref()? {
        BuildRecord::Data { skills } => Some(skills.clone()),
        _ => None,
    }
}
pub(super) fn parse(raw: &str) -> Result<Option<Parsed>, String> {
    let mut build = None;
    let mut level = None;
    let mut skills = Vec::new();
    let mut unknown = Vec::new();
    for section in raw.split("--------") {
        let lines: Vec<_> = section
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if lines.is_empty() {
            continue;
        }
        for line in &lines {
            if let Some(name) = line.strip_prefix("Build: ") {
                let catalogue = super::catalogue();
                let entry = catalogue
                    .iter()
                    .find(|e| e.namespace == "MERCENARY_BUILD" && e.name == name)
                    .ok_or("Unknown Mercenary Build.")?;
                build = Some(match &entry.mercenary_build {
                    Some(BuildRecord::Alias(alias)) => alias.clone(),
                    _ => entry.reference.clone(),
                });
            }
            if let Some(n) = line.strip_prefix("Mercenary Level: ") {
                level = n.parse().ok();
            }
        }
        if skills.len() >= 6 {
            continue;
        }
        let Some(skill) =
            translation::translate(lines[0], "pseudo", Some("Mercenary Warrant"), true)
                .and_then(|m| translation::mercenary_gem(&m.reference))
        else {
            continue;
        };
        let mut group = vec![skill.reference.clone()];
        for line in &lines[1..] {
            let support = translation::translate(line, "pseudo", Some("Mercenary Warrant"), true)
                .and_then(|m| translation::mercenary_gem(&m.reference));
            if let Some(s) = support {
                group.push(s.reference.clone());
            }
            if support.is_none_or(|s| s.meta.synthetic_family && s.meta.tier != Some(3)) {
                unknown.push(UnknownModifier {
                    text: format!("{line} [{}]", lines[0]),
                    kind: "pseudo".into(),
                });
            }
        }
        skills.push(group);
    }
    Ok(build.map(|build| Parsed {
        build,
        level,
        skills,
        unknown,
    }))
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub reference: String,
    pub text: String,
    pub ids: Vec<String>,
    pub tag: String,
    pub disabled: bool,
    pub hidden: Option<String>,
    pub option: Option<u8>,
    pub tier: Option<u8>,
    pub max_tier: Option<bool>,
    pub icon: Option<String>,
    pub exclude: bool,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub families: Vec<Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub group: String,
    pub meta: Filter,
    pub stats: Vec<Filter>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Node {
    Group(Group),
    Stat(Filter),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub nodes: Vec<Node>,
}
fn gem(name: &str) -> &'static MercenaryGem {
    translation::mercenary_gem(name).expect("upstream Mercenary gem reference")
}
fn filter(gem: &MercenaryGem, tag: &str) -> Filter {
    Filter {
        reference: gem.reference.clone(),
        text: gem.text.clone(),
        ids: gem.ids.clone(),
        tag: tag.into(),
        disabled: true,
        hidden: None,
        option: None,
        tier: None,
        max_tier: None,
        icon: None,
        exclude: false,
        value: None,
        min: None,
        families: vec![],
    }
}
fn bad_skill(build: &str, name: &str) -> bool {
    match build {
        "Kineticist" => name == "Kinetic Bolt",
        "Manyshot" => name == "Icicle Rain",
        "Combatant" => ["Wild Strike", "Spectral Helix"].contains(&name),
        _ => false,
    }
}
fn skill_filter(build: &str, skill: &BuildSkill) -> Filter {
    let mut f = filter(gem(&skill.name), &format!("mercenary-{}", skill.kind));
    f.icon = gem(&skill.name).meta.icon.clone();
    if bad_skill(build, &skill.name) {
        f.tag = "brick".into();
    }
    f
}
impl Parsed {
    pub fn preset(&self) -> Result<Preset, String> {
        if self.skills.is_empty() {
            return Err("Description du mercenaire incomplète : compétences absentes.".into());
        }
        let build = build_skills(&self.build).ok_or("Unknown Mercenary Build.")?;
        let mut nodes = Vec::new();
        for raw in &self.skills {
            let Some(name) = raw.first() else {
                continue;
            };
            let skill = build.iter().find(|s| s.name == *name).ok_or_else(|| {
                format!("Mercenary skill {name} does not belong to {}", self.build)
            })?;
            let mut meta = skill_filter(&self.build, skill);
            let raw_supports: Vec<_> = raw[1..].iter().map(|name| gem(name)).collect();
            let mut supports: Vec<_> = raw_supports
                .iter()
                .copied()
                .filter(|s| !s.meta.synthetic_family || s.meta.tier == Some(3))
                .collect();
            if supports.is_empty() {
                if skill.kind == "primary" && raw_supports.is_empty() {
                    meta.hidden = Some("filters.mercenary_primary_no_support".into());
                }
                nodes.push(Node::Stat(meta));
                continue;
            }
            supports.sort_by_key(|s| std::cmp::Reverse(s.meta.tier.unwrap_or(0) & 3));
            let families: Vec<Vec<String>> = gem(name)
                .meta
                .supports
                .iter()
                .map(|name| {
                    let s = gem(name);
                    if s.family.is_empty() {
                        vec![name.clone()]
                    } else {
                        s.family.clone()
                    }
                })
                .collect();
            let mut stats = Vec::new();
            let sixlink = supports.len() == 5 && !families.is_empty();
            if sixlink {
                let count = supports
                    .iter()
                    .filter(|s| {
                        s.meta.tier.is_some_and(|t| t >= 3) || s.family.last() == Some(&s.reference)
                    })
                    .count() as f64;
                let mut f = filter(gem(name), "property");
                f.reference = "6-Link".into();
                f.text = "6-Link".into();
                f.ids = vec!["item.mercenary_6link".into()];
                f.value = Some(count);
                f.min = Some(count);
                f.families = families.clone();
                stats.push(f);
            }
            for support in &supports {
                let tier = support
                    .meta
                    .tier
                    .ok_or("Mercenary support is missing its tier")?;
                let canon = support
                    .meta
                    .canonical
                    .as_deref()
                    .map(gem)
                    .unwrap_or(support);
                let mut f = filter(canon, "mercenary-support");
                f.ids = if support.family.is_empty() || tier == 3 {
                    support.ids.clone()
                } else {
                    support
                        .family
                        .iter()
                        .map(|s| gem(s))
                        .filter(|s| s.meta.tier.is_some_and(|t| t >= tier))
                        .flat_map(|s| s.ids.clone())
                        .collect()
                };
                f.option = Some(1);
                f.tier = Some(tier);
                f.max_tier = Some(tier >= 3 || support.family.last() == Some(&support.reference));
                stats.push(f);
            }
            if sixlink {
                for family in &families {
                    let first = gem(&family[0]);
                    let linked = if let Some(canon) = &first.meta.canonical {
                        raw_supports
                            .iter()
                            .any(|s| s.meta.canonical.as_ref() == Some(canon))
                    } else {
                        raw_supports.iter().any(|s| s.reference == first.reference)
                    };
                    if linked {
                        continue;
                    }
                    let canon = first.meta.canonical.as_deref().map(gem).unwrap_or(first);
                    let mut f = filter(canon, "mercenary-support");
                    f.ids = family.iter().flat_map(|s| gem(s).ids.clone()).collect();
                    f.exclude = true;
                    stats.push(f);
                }
            }
            nodes.push(Node::Group(Group {
                group: "mercenary".into(),
                meta,
                stats,
            }));
        }
        let mut stats = Vec::new();
        for skill in &build {
            if self.skills.iter().any(|s| s.first() == Some(&skill.name)) {
                continue;
            }
            let mut f = skill_filter(&self.build, skill);
            if f.tag == "brick" {
                f.disabled = false;
            }
            stats.push(f);
        }
        let meta = Filter {
            reference: "Not".into(),
            text: "Not".into(),
            ids: vec!["item.not_group".into()],
            tag: "filter-group".into(),
            disabled: stats.iter().all(|f| f.disabled),
            hidden: None,
            option: None,
            tier: None,
            max_tier: None,
            icon: None,
            exclude: false,
            value: None,
            min: None,
            families: vec![],
        };
        nodes.push(Node::Group(Group {
            group: "not".into(),
            meta,
            stats,
        }));
        Ok(Preset { nodes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    fn sample() -> Preset {
        super::super::Item::parse(include_str!("../fixtures/synthetic-mercenary.txt"))
            .unwrap()
            .mercenary
            .unwrap()
            .preset()
            .unwrap()
    }
    fn first_group(p: &mut Preset) -> &mut Group {
        match &mut p.nodes[0] {
            Node::Group(g) => g,
            _ => panic!("Expected supported skill"),
        }
    }
    fn passes(query: &serde_json::Value, ids: &HashSet<String>) -> bool {
        query["stats"].as_array().unwrap().iter().all(|g| {
            let matches = g["filters"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| ids.contains(f["id"].as_str().unwrap()))
                .count();
            match g["type"].as_str().unwrap() {
                "and" => matches == g["filters"].as_array().unwrap().len(),
                "not" => matches == 0,
                "mercenary" | "count" => matches as f64 >= g["value"]["min"].as_f64().unwrap_or(0.),
                _ => false,
            }
        })
    }
    #[test]
    fn optional_supports_allow_one_absent_but_never_replace_the_skill_or_required_support() {
        let mut p = sample();
        let g = first_group(&mut p);
        g.meta.disabled = false;
        let selected: Vec<_> = g
            .stats
            .iter_mut()
            .filter(|f| f.option.is_some())
            .take(3)
            .enumerate()
            .map(|(index, f)| {
                f.disabled = false;
                f.option = Some(if index == 0 { 1 } else { 0 });
                f.ids[0].clone()
            })
            .collect();
        let skill = g.meta.ids[0].clone();
        let mut q = serde_json::json!({"stats":[{"type":"and","filters":[]}]});
        p.append_query(&mut q);
        for optional in [&selected[1], &selected[2]] {
            let ids = HashSet::from([skill.clone(), selected[0].clone(), optional.clone()]);
            assert!(passes(&q, &ids));
            let mut missing_skill = ids.clone();
            missing_skill.remove(&skill);
            assert!(!passes(&q, &missing_skill));
            let mut missing_required = ids;
            missing_required.remove(&selected[0]);
            assert!(!passes(&q, &missing_required));
        }
        assert!(!passes(&q, &HashSet::from([skill, selected[0].clone()])));
    }
    #[test]
    fn excluding_an_unsocketed_support_forces_local_six_links_without_a_global_not_filter() {
        let mut p = sample();
        let g = first_group(&mut p);
        g.meta.disabled = false;
        let excluded = g.stats.iter_mut().find(|f| f.exclude).unwrap();
        excluded.disabled = false;
        let excluded_ids = excluded.ids.clone();
        let mut q = serde_json::json!({"stats":[{"type":"and","filters":[]}]});
        p.append_query(&mut q);
        let groups = q["stats"].as_array().unwrap();
        assert!(groups.iter().any(|g| g["type"] == "mercenary"));
        assert!(groups.iter().all(|g| g["type"] != "not"));
        for filter in groups.iter().flat_map(|g| g["filters"].as_array().unwrap()) {
            assert!(!excluded_ids.iter().any(|id| filter["id"] == *id));
        }
    }
    #[test]
    fn missing_build_is_rejected_for_search_and_unknown_build_is_a_parse_error() {
        let item=super::super::Item::parse("Item Class: Mercenary Warrants\nRarity: Normal\nMercenary Warrant\n--------\nMercenary Level: 83").unwrap();
        assert!(
            crate::filters::query(&item, &crate::filters::Filters::default())
                .unwrap_err()
                .contains("incomplète")
        );
        assert!(super::super::Item::parse("Item Class: Mercenary Warrants\nRarity: Normal\nMercenary Warrant\n--------\nBuild: Not a build").unwrap_err().contains("Unknown Mercenary Build"));
    }
    #[test]
    fn merged_support_ids_and_unknown_synthetic_tiers_are_preserved() {
        assert_eq!(
            gem("Gilded Extra Targets").ids,
            vec!["mercenary.support_37259", "mercenary.support_58471"]
        );
        let text = include_str!("../fixtures/synthetic-mercenary.txt").replace(
            "\n--------\nTempest Shield",
            "\nLeech (Tier: 2)\n--------\nTempest Shield",
        );
        let item = super::super::Item::parse(&text).unwrap();
        assert!(item
            .unknown_modifiers
            .iter()
            .any(|m| m.text == "Leech (Tier: 2) [Shield Crush]"));
        let mut p = item.mercenary.unwrap().preset().unwrap();
        assert!(first_group(&mut p)
            .stats
            .iter()
            .all(|f| f.reference != "Leech"));
    }
    #[test]
    fn build_without_skill_sections_cannot_search_all_mercenaries() {
        let item=super::super::Item::parse("Item Class: Mercenary Warrants\nRarity: Normal\nMercenary Warrant\n--------\nMercenary Level: 83\nBuild: Bastion").unwrap();
        assert!(
            crate::filters::query(&item, &crate::filters::Filters::default())
                .unwrap_err()
                .contains("compétences absentes")
        );
    }
    #[test]
    fn primary_skill_is_retained_before_any_support_selection() {
        let p = sample();
        let skill = match &p.nodes[0] {
            Node::Group(g) => g.meta.ids[0].clone(),
            _ => panic!("Expected group"),
        };
        let mut q = serde_json::json!({"stats":[{"type":"and","filters":[]}]});
        p.append_query(&mut q);
        assert_eq!(q["stats"][0]["filters"][0]["id"], skill);
        assert_eq!(q["stats"].as_array().unwrap().len(), 1);
    }
}
