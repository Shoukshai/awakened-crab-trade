//! WidgetDelveGrid.vue geometry and read-only visibility policy.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Geometry {
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub left: f64,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub cell: i32,
    pub grid_left: f64,
    pub grid_top: f64,
    pub rows: usize,
    pub columns: usize,
}
pub const BORDER_RGB: [u8; 3] = [0x71, 0x80, 0x96];
impl Geometry {
    pub fn new(viewport_width: i32, viewport_height: i32) -> Result<Self, String> {
        if viewport_width <= 0 || viewport_height <= 0 {
            return Err("La grille exige une taille de sortie positive.".into());
        }
        // Positive integer version of JS Math.round; no application scale is
        // applied, since upstream aligns the grid with the game viewport.
        let round = |n: i64, d: i64| ((n + d / 2) / d) as i32;
        let height = round(i64::from(viewport_height) * 808, 1080);
        let width = round(i64::from(height) * 1030, 808);
        let top = round(i64::from(viewport_height) * 67, 1080);
        let cell = round(i64::from(viewport_height) * 97, 1080);
        let left = f64::from(viewport_width - width) / 2.;
        Ok(Self {
            viewport_width,
            viewport_height,
            left,
            top,
            width,
            height,
            cell,
            grid_left: left + f64::from(width - cell * 11) / 2.,
            grid_top: f64::from(top) + f64::from(height - cell * 9) / 2.,
            rows: 9,
            columns: 11,
        })
    }
}
#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Hidden,
    Preparing,
    Passive,
    Error,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum Reason {
    Toggle,
    Explicit,
    Interactive,
    OtherApplication,
    GameGone,
    GameInactive,
    TrackerFailure,
    Quit,
    DrawError,
}
#[derive(Debug, Clone, Default, Serialize)]
pub struct Policy {
    pub phase: Phase,
    pub generation: u64,
    pub waiting_for_game: bool,
    pub last_hide: Option<Reason>,
}
impl Policy {
    pub fn requested(&self) -> bool {
        matches!(self.phase, Phase::Preparing | Phase::Passive)
    }
    pub fn toggle(&mut self, allowed: bool, wait_for_game: bool) -> bool {
        if self.requested() {
            self.hide(Reason::Toggle);
            return false;
        }
        if !allowed {
            return false;
        }
        self.generation += 1;
        self.phase = Phase::Preparing;
        self.waiting_for_game = wait_for_game;
        true
    }
    pub fn hide(&mut self, reason: Reason) {
        if self.phase != Phase::Hidden {
            self.generation += 1;
            self.phase = Phase::Hidden;
            self.waiting_for_game = false;
            self.last_hide = Some(reason);
        }
    }
    pub fn context(
        &mut self,
        game_present: bool,
        game_active: bool,
        other_active: bool,
        interactive: bool,
        exclusive: bool,
    ) -> bool {
        let reason = if !game_present {
            Some(Reason::GameGone)
        } else if other_active {
            Some(Reason::OtherApplication)
        } else if interactive {
            Some(Reason::Interactive)
        } else if !game_active && !self.waiting_for_game {
            Some(Reason::GameInactive)
        } else {
            None
        };
        if let Some(reason) = reason {
            self.hide(reason);
            return false;
        }
        if game_active {
            self.waiting_for_game = false;
        }
        self.requested() && game_active && !exclusive
    }
    pub fn painted(&mut self, generation: u64) -> bool {
        if self.requested() && self.generation == generation && !self.waiting_for_game {
            self.phase = Phase::Passive;
            true
        } else {
            false
        }
    }
    pub fn fail(&mut self) {
        self.hide(Reason::DrawError);
        self.phase = Phase::Error;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn game_proportions_and_intentional_clipping_are_kept() {
        let g = Geometry::new(1920, 1080).unwrap();
        assert_eq!((g.width, g.height, g.top, g.cell), (1030, 808, 67, 97));
        assert_eq!((g.left, g.grid_left, g.grid_top), (445., 426.5, 34.5));
        let g = Geometry::new(2560, 1600).unwrap();
        assert_eq!((g.width, g.height, g.top, g.cell), (1526, 1197, 99, 144));
        assert_eq!((g.left, g.grid_left, g.grid_top), (517., 488., 49.5));
        assert!(Geometry::new(0, 1600).is_err());
        assert!(Geometry::new(2560, -1).is_err());
    }
    #[test]
    fn interaction_context_loss_and_old_frames_cannot_reopen_the_grid() {
        let mut p = Policy::default();
        assert!(!p.toggle(false, false));
        assert!(p.toggle(true, false));
        assert!(p.context(true, true, false, false, false));
        let frame = p.generation;
        assert!(!p.context(true, true, false, false, true));
        assert!(p.requested());
        assert!(p.context(true, true, false, false, false));
        assert!(p.painted(frame));
        assert_eq!(p.phase, Phase::Passive);
        assert!(!p.context(true, false, false, true, false));
        assert_eq!(p.last_hide, Some(Reason::Interactive));
        assert!(!p.painted(frame));
        assert!(!p.context(true, true, false, false, false));
        assert!(p.toggle(true, true));
        assert!(!p.context(true, false, false, false, false));
        assert!(p.context(true, true, false, false, false));
        assert!(!p.waiting_for_game);
        assert!(!p.context(true, false, true, false, false));
        assert_eq!(p.last_hide, Some(Reason::OtherApplication));
        assert!(!p.context(true, true, false, false, false));
    }
}
