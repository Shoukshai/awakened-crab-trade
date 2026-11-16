//! Section-level parsers from Parser.ts: logbook areas, temple rooms and tablets.
use crate::modifiers::{self, translation, Modifier, UnknownModifier};
#[derive(Default)]
pub(super) struct Parsed {
    pub modifiers: Vec<Modifier>,
    pub unknown: Vec<UnknownModifier>,
    pub logbook_areas: Vec<Vec<Modifier>>,
}
fn merge_max(out: &mut Vec<Modifier>, modifier: Modifier) {
    if let Some(total) = out.iter_mut().find(|m| m.reference == modifier.reference) {
        total.value = total.value.zip(modifier.value).map(|(a, b)| a.max(b));
        total.min = total.min.zip(modifier.min).map(|(a, b)| a.max(b));
        total.max = total.max.zip(modifier.max).map(|(a, b)| a.max(b));
        total.source_count += modifier.source_count;
    } else {
        out.push(modifier);
    }
}
pub(super) fn parse(raw: &str, reference: &str, category: Option<&str>) -> Parsed {
    let mut parsed = Parsed::default();
    let mut remaining = Vec::new();
    let mut temple_parsed = false;
    let mut tablet_parsed = false;
    for section in raw.split("--------") {
        let lines: Vec<_> = section
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if reference == "Filled Coffin" && lines.iter().any(|line| line.ends_with(" (implicit)")) {
            for line in &lines {
                let line = line.strip_suffix(" (implicit)").unwrap_or(line);
                if let Some(m) = translation::translate(line, "necropolis", category, false) {
                    parsed.modifiers.push(m);
                } else if !line.starts_with('{') {
                    parsed.unknown.push(UnknownModifier {
                        text: line.to_string(),
                        kind: "necropolis".into(),
                    });
                }
            }
            continue;
        }
        if reference == "Expedition Logbook" && lines.len() >= 3 && parsed.logbook_areas.len() < 3 {
            if let Some(faction) = translation::translate(lines[1], "pseudo", None, false)
                .filter(|m| m.reference.starts_with("Has Logbook Faction:"))
            {
                let mut area = vec![faction];
                // Only non-comparable area mods (boss/faction) affect these presets.
                // parse_detailed preserves the ordinary/implicit advanced mod type.
                let (mods, _) = modifiers::parse_detailed(&lines[2..].join("\n"), category);
                area.extend(mods.into_iter().filter(|m| m.better == 0));
                parsed.logbook_areas.push(area);
                continue;
            }
        }
        if reference == "Chronicle of Atzoatl"
            && !temple_parsed
            && lines.first() == Some(&"Open Rooms:")
        {
            temple_parsed = true;
            let mut state = 1.;
            for line in &lines[1..] {
                if *line == "Obstructed Rooms:" {
                    state = 2.;
                    continue;
                }
                if let Some(mut m) = translation::translate(line, "pseudo", None, true) {
                    m.value = Some(state);
                    m.min = Some(state);
                    m.max = Some(state);
                    if state == 2. {
                        m.text = format!("Obstructed Rooms: {line}");
                    }
                    parsed.modifiers.push(m);
                } else {
                    parsed.unknown.push(UnknownModifier {
                        text: line.to_string(),
                        kind: "pseudo".into(),
                    });
                }
            }
            continue;
        }
        if reference == "Mirrored Tablet" && !tablet_parsed && lines.len() >= 8 {
            tablet_parsed = true;
            for line in lines {
                if let Some(m) = translation::translate(line, "pseudo", None, true) {
                    merge_max(&mut parsed.modifiers, m);
                } else {
                    parsed.unknown.push(UnknownModifier {
                        text: line.to_string(),
                        kind: "pseudo".into(),
                    });
                }
            }
            continue;
        }
        remaining.push(section);
    }
    let (mods, unknown) = modifiers::parse_detailed(&remaining.join("--------"), category);
    parsed.modifiers.extend(mods);
    parsed.unknown.extend(unknown);
    parsed
}
