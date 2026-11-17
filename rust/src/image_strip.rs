//! Portable image-strip configuration and upstream image/anchor operations.
use serde::{Deserialize, Serialize};
pub const MAX_STRIPS: usize = 16;
pub const MAX_IMAGES: usize = 32;
pub const MAX_ID: u64 = 9_007_199_254_740_991;
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    Tl,
    #[default]
    Tc,
    Tr,
    Bl,
    Bc,
    Br,
}
impl Anchor {
    pub const ALL: [Self; 6] = [Self::Tl, Self::Tc, Self::Tr, Self::Bl, Self::Bc, Self::Br];
    pub fn fractions(self) -> (f64, f64) {
        match self {
            Self::Tl => (0., 0.),
            Self::Tc => (0.5, 0.),
            Self::Tr => (1., 0.),
            Self::Bl => (0., 1.),
            Self::Bc => (0.5, 1.),
            Self::Br => (1., 1.),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Image {
    pub id: u64,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub id: u64,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub anchor: Anchor,
    pub wanted: bool,
    pub images: Vec<Image>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            id: 1,
            title: "Cheat sheets".into(),
            x: 50.,
            y: 10.,
            anchor: Anchor::Tc,
            wanted: false,
            images: vec![Image {
                id: 1,
                url: "syndicate.jpg".into(),
            }],
        }
    }
}
impl Config {
    pub fn new(id: u64, random_x: f64, random_y: f64) -> Self {
        Self {
            id,
            title: String::new(),
            x: 40. + random_x * 20.,
            y: 5. + random_y * 10.,
            wanted: true,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.id == 0 || self.id > MAX_ID || self.title.chars().count() > 100 {
            return Err("Identifiant ou titre de bande d’images invalide.".into());
        }
        if !self.x.is_finite()
            || !self.y.is_finite()
            || !(0. ..=100.).contains(&self.x)
            || !(0. ..=100.).contains(&self.y)
        {
            return Err("Position de bande d’images invalide.".into());
        }
        if self.images.len() > MAX_IMAGES {
            return Err("Au plus 32 images par bande.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for image in &self.images {
            if image.id == 0 || image.id > MAX_ID || !ids.insert(image.id) || !valid_url(&image.url)
            {
                return Err("Référence d’image invalide ou identifiant dupliqué.".into());
            }
        }
        Ok(())
    }
    pub fn append(&mut self, url: String) -> Result<u64, String> {
        if !valid_url(&url) || self.images.len() >= MAX_IMAGES {
            return Err("Image ou nombre d’images invalide.".into());
        }
        let id = self.images.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        if id > MAX_ID {
            return Err("Identifiants d’image épuisés.".into());
        }
        self.images.push(Image { id, url });
        Ok(id)
    }
    pub fn remove(&mut self, id: u64) {
        self.images.retain(|i| i.id != id);
    }
    pub fn reorder(&mut self, ids: &[u64]) -> Result<(), String> {
        if ids.len() != self.images.len() {
            return Err("Ordre d’images incomplet.".into());
        }
        let mut seen = std::collections::HashSet::new();
        let mut sorted = Vec::new();
        for id in ids {
            if !seen.insert(id) {
                return Err("Ordre d’images dupliqué.".into());
            }
            sorted.push(
                self.images
                    .iter()
                    .find(|i| i.id == *id)
                    .ok_or("Image absente de la bande.")?
                    .clone(),
            );
        }
        self.images = sorted;
        Ok(())
    }
    pub fn move_to(
        &mut self,
        anchor: Anchor,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), String> {
        if ![x, y, width, height].iter().all(|x| x.is_finite()) || width <= 0. || height <= 0. {
            return Err("Déplacement d’image invalide.".into());
        }
        self.anchor = anchor;
        self.x = (x / width).clamp(0., 1.) * 100.;
        self.y = (y / height).clamp(0., 1.) * 100.;
        Ok(())
    }
    pub fn origin(
        &self,
        width: f64,
        height: f64,
        widget_width: f64,
        widget_height: f64,
    ) -> (f64, f64) {
        let (fx, fy) = self.anchor.fractions();
        (
            self.x / 100. * width - fx * widget_width,
            self.y / 100. * height - fy * widget_height,
        )
    }
}
/// Native uploads are immutable content-addressed local files. No arbitrary path
/// is interpreted as an image URL. The built-in upstream sheet is embedded.
pub fn valid_url(url: &str) -> bool {
    if url == "syndicate.jpg" {
        return true;
    }
    let Some((hash, ext)) = url.split_once('.') else {
        return false;
    };
    hash.len() == 64
        && hash
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        && !ext.is_empty()
        && ext.len() <= 8
        && ext.bytes().all(|c| c.is_ascii_alphanumeric())
}
pub fn defaults() -> Vec<Config> {
    vec![Config::default()]
}
pub fn is_default(value: &[Config]) -> bool {
    *value == defaults()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_order_is_transactional_and_ids_follow_upstream_max() {
        let mut c = Config::default();
        let url = format!("{}.png", "a".repeat(64));
        assert_eq!(c.append(url.clone()).unwrap(), 2);
        assert_eq!(c.append(url).unwrap(), 3);
        c.remove(2);
        c.reorder(&[3, 1]).unwrap();
        assert_eq!(c.images[0].id, 3);
        let previous = c.clone();
        assert!(c.reorder(&[1, 1]).is_err());
        assert_eq!(c, previous);
        assert!(!valid_url("../../file.png"));
        assert!(!valid_url("https://example.com/p.png"));
        c.validate().unwrap();
    }
    #[test]
    fn anchors_clamp_and_default_visibility_matches_image_strip() {
        let mut c = Config::default();
        assert!(!c.wanted);
        assert_eq!(c.origin(2560., 1600., 400., 150.), (1080., 160.));
        c.move_to(Anchor::Br, 3000., -50., 2560., 1600.).unwrap();
        assert_eq!((c.x, c.y), (100., 0.));
        assert_eq!(c.origin(2560., 1600., 400., 150.), (2160., -150.));
        let c = Config::new(2, 0., 0.999);
        assert_eq!(c.x, 40.);
        assert!(c.y < 15. && c.wanted);
    }
}

/// FullscreenImage.vue hover guard. Coordinates are in one common viewport.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Hover {
    pub hovered: bool,
    pub requires_leave: bool,
}
impl Hover {
    pub fn motion(
        &mut self,
        disabled: bool,
        rect: [f64; 4],
        position: [f64; 2],
        movement: [f64; 2],
    ) {
        if disabled || self.hovered || self.requires_leave {
            return;
        }
        let px = position[0] - movement[0];
        let py = position[1] - movement[1];
        if px <= rect[0] + 2. || px >= rect[2] - 2. || py <= rect[1] + 2. || py >= rect[3] - 2. {
            self.hovered = true;
        } else {
            self.requires_leave = true;
        }
    }
    pub fn leave(&mut self) {
        self.hovered = false;
        self.requires_leave = false;
    }
    pub fn click(&mut self, disabled: bool) {
        if !disabled {
            self.hovered = true;
        }
    }
}
