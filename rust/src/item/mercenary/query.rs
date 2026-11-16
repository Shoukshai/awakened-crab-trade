//! Effective Trade groups from pathofexile-trade.ts (disabled nodes omitted).
use super::{gem, Filter, Node, Preset};
use serde_json::{json, Value};
fn values(f: &Filter) -> Vec<Value> {
    f.ids.iter().map(|id| json!({"id":id,"value":{}})).collect()
}
fn weighted(all: Vec<Vec<String>>, some: Vec<Vec<String>>, minimum: f64) -> Value {
    let weight = (some.len() as f64 - minimum).max(0.) + 1.;
    let mut filters = Vec::new();
    for family in &all {
        for id in family {
            for _ in 0..weight.ceil() as usize {
                filters.push(json!({"id":id}));
            }
        }
    }
    filters.extend(some.into_iter().flatten().map(|id| json!({"id":id})));
    json!({"type":"mercenary","value":{"min":minimum+all.len() as f64*weight},"filters":filters})
}
impl Preset {
    pub fn append_query(&self, query: &mut Value) {
        let mut and = Vec::new();
        let mut extra = Vec::new();
        let mut not = Vec::new();
        let mut plain = Vec::new();
        for node in &self.nodes {
            let group = match node {
                Node::Stat(f) => {
                    plain.push(f);
                    continue;
                }
                Node::Group(g) => g,
            };
            if group.group == "not" {
                if !group.meta.disabled {
                    let filters: Vec<_> = group
                        .stats
                        .iter()
                        .filter(|f| !f.disabled)
                        .flat_map(values)
                        .collect();
                    extra.push(json!({"type":"not","filters":filters}));
                }
                continue;
            }
            let skill = &group.meta;
            let stats = &group.stats;
            if skill.tag == "mercenary-primary" || !skill.disabled {
                and.extend(values(skill));
            }
            let socketed: Vec<_> = stats
                .iter()
                .filter(|f| !f.exclude && f.ids.first().is_none_or(|s| s != "item.mercenary_6link"))
                .collect();
            let optional: Vec<_> = socketed
                .iter()
                .filter(|f| !f.disabled && f.option == Some(0))
                .collect();
            let required: Vec<_> = socketed
                .iter()
                .filter(|f| !f.disabled && f.option == Some(1))
                .collect();
            let local = stats
                .iter()
                .any(|f| f.ids.first().is_some_and(|s| s == "item.mercenary_6link"));
            let local_not: Vec<_> = stats
                .iter()
                .filter(|f| local && f.exclude && !f.disabled)
                .collect();
            for f in stats {
                if skill.disabled || required.len() == 5 {
                    break;
                }
                if f.exclude {
                    if !local && !f.disabled {
                        not.extend(values(f));
                    }
                    continue;
                }
                if f.ids.first().is_none_or(|s| s != "item.mercenary_6link") {
                    continue;
                }
                let force = f.disabled && !local_not.is_empty();
                if f.disabled && !force {
                    continue;
                }
                let possible: Vec<_> = f
                    .families
                    .iter()
                    .filter(|family| {
                        let first = gem(&family[0]);
                        !local_not.iter().any(|f| {
                            Some(&f.reference) == first.meta.canonical.as_ref()
                                || f.reference == first.reference
                        })
                    })
                    .collect();
                let mut count = f.min.unwrap_or(0.).clamp(0., 5.);
                let max_enabled = required.iter().filter(|f| f.max_tier == Some(true)).count()
                    as i64
                    + if optional.len() >= 2 {
                        optional.iter().filter(|f| f.max_tier == Some(true)).count() as i64 - 1
                    } else {
                        0
                    };
                if force || max_enabled as f64 >= count {
                    count = 0.;
                }
                if count < 5. {
                    let ids = possible
                        .iter()
                        .map(|family| {
                            let minimum = if gem(&family[0]).meta.synthetic_family {
                                3
                            } else {
                                2
                            };
                            family
                                .iter()
                                .map(|s| gem(s))
                                .filter(|s| {
                                    family.len() < 3 || s.meta.tier.is_some_and(|t| t >= minimum)
                                })
                                .flat_map(|s| s.ids.clone())
                                .collect()
                        })
                        .collect();
                    extra.push(weighted(vec![skill.ids.clone()], ids, 5.));
                }
                if count > 0. {
                    let ids = possible
                        .iter()
                        .map(|family| gem(family.last().unwrap()).ids.clone())
                        .collect();
                    extra.push(weighted(vec![skill.ids.clone()], ids, count));
                }
            }
            if optional.len() >= 2 {
                let mut all = vec![skill.ids.clone()];
                all.extend(required.iter().map(|f| f.ids.clone()));
                extra.push(weighted(
                    all,
                    optional.iter().map(|f| f.ids.clone()).collect(),
                    (optional.len() - 1) as f64,
                ));
            } else if !skill.disabled && !required.is_empty() {
                let mut filters = values(skill);
                filters.extend(required.iter().flat_map(|f| values(f)));
                extra.push(
                    json!({"type":"mercenary","value":{"min":1+required.len()},"filters":filters}),
                );
            }
        }
        for f in plain {
            if f.tag == "mercenary-primary" || !f.disabled {
                if f.exclude {
                    not.extend(values(f));
                } else {
                    and.extend(values(f));
                }
            }
        }
        if !not.is_empty() {
            extra.push(json!({"type":"not","filters":not}));
        }
        let groups = query["stats"]
            .as_array_mut()
            .expect("Trade query stat groups");
        groups[0]["filters"].as_array_mut().unwrap().extend(and);
        groups.extend(extra);
    }
}
