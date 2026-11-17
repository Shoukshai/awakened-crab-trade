//! WidgetTimer.vue state. Product callers supply a monotonic millisecond clock;
//! deterministic callers can reproduce the unchanged upstream transitions.
use serde::{Deserialize, Serialize};
pub const MAX_INSTANCES: usize = 32;
pub const MAX_ID: u64 = 9_007_199_254_740_991;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub id: u64,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub wanted: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            id: 1,
            title: String::new(),
            x: 50.,
            y: 50.,
            wanted: true,
        }
    }
}
impl Config {
    pub fn new(id: u64, random_x: f64, random_y: f64) -> Self {
        Self {
            id,
            x: 40. + random_x * 20.,
            y: 40. + random_y * 20.,
            ..Default::default()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.id == 0 || self.id > MAX_ID {
            return Err("Identifiant de chronomètre invalide.".into());
        }
        if self.title.chars().count() > 100 {
            return Err("Le nom du chronomètre est limité à 100 caractères.".into());
        }
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !(0. ..=100.).contains(&self.x)
            || !(0. ..=100.).contains(&self.y)
        {
            return Err("La position du chronomètre doit être comprise entre 0 et 100 %.".into());
        }
        Ok(())
    }
    pub fn move_to(&mut self, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
        if !x.is_finite()
            || !y.is_finite()
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.
            || height <= 0.
        {
            return Err("Position de déplacement invalide.".into());
        }
        self.x = (x / width).clamp(0., 1.) * 100.;
        self.y = (y / height).clamp(0., 1.) * 100.;
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Reading {
    pub id: u64,
    pub running: bool,
    pub millis: u64,
    pub formatted: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub running: bool,
    pub millis: u64,
    pub previous_tick: u64,
    pub invisible_on_blur: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            running: false,
            millis: 0,
            previous_tick: 0,
            invisible_on_blur: true,
        }
    }
}
impl State {
    pub fn start(&mut self, now: u64) {
        self.running = true;
        self.previous_tick = now;
        self.invisible_on_blur = false;
    }
    pub fn stop(&mut self, now: u64) {
        self.update(now);
        self.running = false;
        if self.millis < 1000 {
            self.invisible_on_blur = true;
        }
    }
    pub fn toggle(&mut self, now: u64) {
        if self.running {
            self.stop(now)
        } else {
            self.start(now)
        }
    }
    pub fn reset(&mut self, now: u64) {
        self.previous_tick = now;
        self.millis = if self.running { 1000 } else { 0 };
        if !self.running {
            self.invisible_on_blur = true;
        }
    }
    pub fn update(&mut self, now: u64) {
        if self.running {
            self.millis = self.elapsed(now);
            self.previous_tick = now;
        }
    }
    pub fn elapsed(&self, now: u64) -> u64 {
        if self.running {
            self.millis
                .saturating_add(now.saturating_sub(self.previous_tick))
        } else {
            self.millis
        }
    }
    pub fn visible(&self, wanted: bool, interactive: bool, exclusive: bool) -> bool {
        wanted && !exclusive && (interactive || !self.invisible_on_blur)
    }
    pub fn formatted(millis: u64) -> String {
        format!(
            "{:02}:{:02}:{:02}",
            millis / 3_600_000,
            (millis / 60_000) % 60,
            (millis / 1000) % 60
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reset_running_one_second_and_paused_visibility_match_upstream() {
        let mut s = State::default();
        assert!(!s.visible(true, false, false));
        s.start(500);
        s.stop(1499);
        assert_eq!(s.millis, 999);
        assert!(s.invisible_on_blur);
        s.start(2000);
        s.stop(2001);
        assert_eq!(s.millis, 1000);
        assert!(!s.invisible_on_blur);
        assert!(s.visible(true, false, false));
        assert!(!s.visible(true, false, true));
        s.reset(3000);
        assert_eq!(s.millis, 0);
        assert!(s.invisible_on_blur);
        s.start(4000);
        s.reset(4500);
        assert_eq!(s.millis, 1000);
        assert!(s.running);
        s.stop(5000);
        assert_eq!(s.millis, 1500);
    }
    #[test]
    fn hidden_running_time_is_lazy_and_instances_are_independent() {
        let mut a = State::default();
        let b = State::default();
        a.start(10);
        assert_eq!(a.elapsed(3_600_025), 3_600_015);
        assert_eq!(a.millis, 0);
        assert_eq!(b.elapsed(3_600_025), 0);
        a.stop(3_600_025);
        assert_eq!(State::formatted(a.millis), "01:00:00");
        assert_eq!(State::formatted(100 * 3_600_000 + 61_999), "100:01:01");
    }
    #[test]
    fn positions_are_center_anchored_clamped_and_reject_nonfinite_input() {
        let mut c = Config::new(2, 0., 0.999);
        assert_eq!(c.x, 40.);
        assert!(c.y < 60.);
        c.move_to(1280., 800., 2560., 1600.).unwrap();
        assert_eq!((c.x, c.y), (50., 50.));
        c.move_to(-1., 1700., 2560., 1600.).unwrap();
        assert_eq!((c.x, c.y), (0., 100.));
        assert!(c.move_to(f64::NAN, 0., 1., 1.).is_err());
        c.id = 0;
        assert!(c.validate().is_err());
    }
}
