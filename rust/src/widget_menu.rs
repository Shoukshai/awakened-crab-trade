//! Portable WidgetMenu.vue configuration, filtering, and title convention.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    #[default]
    Tl,
    Tr,
    Bl,
    Br,
}
impl Anchor {
    pub const ALL: [Self; 4] = [Self::Tl, Self::Tr, Self::Bl, Self::Br];
    pub fn fractions(self) -> (f64, f64) {
        match self {
            Self::Tl => (0., 0.),
            Self::Tr => (1., 0.),
            Self::Bl => (0., 1.),
            Self::Br => (1., 1.),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub x: f64,
    pub y: f64,
    pub anchor: Anchor,
    pub always_show: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            x: 5.,
            y: 5.,
            anchor: Anchor::Tl,
            always_show: false,
        }
    }
}
impl Config {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !(0. ..=100.).contains(&self.x)
            || !(0. ..=100.).contains(&self.y)
        {
            Err("Position du menu invalide.".into())
        } else {
            Ok(())
        }
    }
    pub fn origin(&self, w: f64, h: f64, ww: f64, wh: f64) -> (f64, f64) {
        let (x, y) = self.anchor.fractions();
        (self.x / 100. * w - x * ww, self.y / 100. * h - y * wh)
    }
    pub fn move_to(
        &mut self,
        anchor: Anchor,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) -> Result<(), String> {
        if ![x, y, w, h].iter().all(|n| n.is_finite()) || w <= 0. || h <= 0. {
            return Err("Déplacement du menu invalide.".into());
        }
        self.anchor = anchor;
        self.x = (x / w).clamp(0., 1.) * 100.;
        self.y = (y / h).clamp(0., 1.) * 100.;
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum Target {
    Settings,
    Catalogue,
    Stopwatch(u64),
    Image(u64),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub target: Target,
    pub id: u64,
    pub title: String,
    pub wanted: bool,
    pub skip: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Label {
    pub target: Target,
    pub wanted: bool,
    pub title: Option<String>,
    pub icon: Option<String>,
}
pub fn entries(items: &[Entry], always: bool) -> Vec<Label> {
    use std::sync::OnceLock;
    // JavaScript whitespace, including BOM and excluding U+0085. Its dot also
    // excludes CR and Unicode line separators. No user pattern is evaluated.
    static TITLE: OnceLock<regex::Regex> = OnceLock::new();
    let re=TITLE.get_or_init(||regex::Regex::new(r"^(?:\{icon=(?P<icon>[^}]+)\}[\x09-\x0d\x20\u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*)?(?P<title>[^\n\r\u{2028}\u{2029}]*)\z").unwrap());
    items
        .iter()
        .filter(|e| !e.skip && (always || !e.wanted))
        .map(|e| {
            let captures = re.captures(&e.title);
            let icon = captures
                .as_ref()
                .and_then(|c| c.name("icon"))
                .map(|s| s.as_str().to_string());
            let title = captures
                .as_ref()
                .and_then(|c| c.name("title"))
                .filter(|s| !s.as_str().is_empty())
                .map(|s| s.as_str().to_string())
                .or_else(|| icon.is_none().then(|| format!("#{}", e.id)));
            Label {
                target: e.target.clone(),
                wanted: e.wanted,
                title,
                icon,
            }
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visibility_and_titles_preserve_icon_only_and_skip_rules() {
        let items = [
            Entry {
                target: Target::Settings,
                id: 1,
                title: "{icon=fa-cog}".into(),
                wanted: false,
                skip: false,
            },
            Entry {
                target: Target::Image(1),
                id: 7,
                title: String::new(),
                wanted: true,
                skip: false,
            },
            Entry {
                target: Target::Catalogue,
                id: 8,
                title: "hidden".into(),
                wanted: false,
                skip: true,
            },
        ];
        let visible = entries(&items, false);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].title, None);
        assert_eq!(visible[0].icon.as_deref(), Some("fa-cog"));
        assert_eq!(entries(&items, true)[1].title.as_deref(), Some("#7"));
    }
    #[test]
    fn corner_anchor_is_bounded_and_old_profiles_have_the_upstream_default() {
        let mut c: Config = serde_json::from_str("{}").unwrap();
        assert!(c.is_default());
        c.move_to(Anchor::Br, 3000., -10., 2560., 1600.).unwrap();
        assert_eq!((c.x, c.y), (100., 0.));
        assert_eq!(c.origin(2560., 1600., 400., 100.), (2160., -100.));
        assert!(c.validate().is_ok());
    }
}
