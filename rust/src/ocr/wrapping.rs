//! Reassemble visual wraps only when the entire text is a catalogue modifier.
use super::{key, modifier_header, readings, reminder, stat, Line};

fn boundary(line: &Line) -> bool {
    modifier_header(&line.text).is_some()
        || readings::property(&line.text).is_some()
        || super::is_requirement(&line.text)
        || reminder(&line.text)
        || ["corrupted", "mirrored", "unidentified"].contains(&key(&line.text).as_str())
        || line.text.starts_with("Conflicting OCR readings:")
}

pub(super) fn repair(lines: &mut Vec<Line>) {
    lines.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    let mut i = 0;
    while i < lines.len() {
        if boundary(&lines[i]) {
            i += 1;
            continue;
        }
        let first = &lines[i];
        let complete = stat(&first.text).is_some();
        let center = first.x + first.width / 2.;
        let mut joined = first.text.clone();
        let mut found = Vec::new();
        for j in i + 1..lines.len().min(i + 8) {
            let previous = &lines[j - 1];
            let next = &lines[j];
            let height = previous.height.max(next.height).max(1.);
            // Never consume another complete mod, source header, independent
            // column, conflicting reading or a separated tooltip section.
            if boundary(next)
                || (complete && stat(&next.text).is_some())
                || first
                    .source
                    .as_ref()
                    .zip(next.source.as_ref())
                    .is_some_and(|(a, b)| a != b)
                || next.y < previous.y + previous.height * 0.5
                || next.y - previous.y - previous.height > height * 1.1
                || (next.x + next.width / 2. - center).abs() > (first.width * 0.4).max(42.)
            {
                break;
            }
            joined.push(' ');
            joined.push_str(next.text.trim());
            if joined.len() > 2048 {
                break;
            }
            if let Some(text) = stat(&joined) {
                found.push((j, text));
            }
            // A standalone suffix such as "10% increased Spell Damage" can
            // complete a cluster's "Added Small Passive Skills grant:" line,
            // but must never become a bridge to the following modifier.
            if stat(&next.text).is_some() {
                break;
            }
        }
        // Ambiguous endpoints stay in review. No words or numbers are supplied
        // from a guessed modifier and no unrelated lines are discarded.
        if let [(end, text)] = found.as_slice() {
            let end = *end;
            let left = lines[i..=end]
                .iter()
                .map(|l| l.x)
                .fold(f64::INFINITY, f64::min);
            let right = lines[i..=end]
                .iter()
                .map(|l| l.x + l.width)
                .fold(0., f64::max);
            let bottom = lines[end].y + lines[end].height;
            let merged = Line {
                text: text.clone(),
                x: left,
                y: lines[i].y,
                width: right - left,
                height: bottom - lines[i].y,
                rarity: lines[i].rarity.clone(),
                source: lines[i].source.clone(),
                readings: Vec::new(),
            };
            lines.splice(i..=end, [merged]);
        }
        i += 1;
    }
}
