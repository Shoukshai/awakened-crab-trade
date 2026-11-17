//! Ctrl+wheel policy from Shortcuts.ts and GameWindow.uiSidebarWidth.
use crate::hotkeys::TabDirection;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
pub fn is_stash_area(x: f64, y: f64, bounds: Option<Bounds>) -> bool {
    let Some(b) = bounds else {
        return false;
    };
    if x > b.x + (b.height * (370. / 600.)).round() {
        return false;
    }
    y > b.y + b.height * 154. / 1600. && y < b.y + b.height * 1192. / 1600.
}
pub fn action(
    ctrl: bool,
    active: bool,
    enabled: bool,
    x: f64,
    y: f64,
    rotation: i32,
    bounds: Option<Bounds>,
) -> Option<TabDirection> {
    if !ctrl || !active || !enabled || is_stash_area(x, y, bounds) {
        None
    } else if rotation > 0 {
        Some(TabDirection::Right)
    } else if rotation < 0 {
        Some(TabDirection::Left)
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stash_edges_are_open_vertically_and_sidebar_rounding_matches_upstream() {
        let b = Some(Bounds {
            x: 100.,
            y: 200.,
            width: 2560.,
            height: 1600.,
        });
        assert!(is_stash_area(100., 355., b));
        assert!(!is_stash_area(100., 354., b));
        assert!(!is_stash_area(100., 1392., b));
        assert!(is_stash_area(1087., 800., b));
        assert!(!is_stash_area(1088., 800., b));
    }
    #[test]
    fn disabled_or_unmodified_wheels_never_generate_keys() {
        for (ctrl, active, enabled) in [
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            assert_eq!(action(ctrl, active, enabled, 2000., 600., 1, None), None);
        }
        assert_eq!(
            action(true, true, true, 2000., 600., -1, None),
            Some(TabDirection::Left)
        );
    }
}
