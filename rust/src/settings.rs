use crate::filters::Filters;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub widget_order: Vec<crate::widget_order::Key>,
    #[serde(skip_serializing_if = "crate::widget_menu::Config::is_default")]
    pub widget_menu: crate::widget_menu::Config,
    pub league: String,
    pub filters: Filters,
    pub scale: f64,
    pub search_range: f64,
    #[serde(skip_serializing_if = "is_true")]
    pub automatic_search: bool,
    pub map_check: crate::inspection::MapConfig,
    #[serde(skip_serializing_if = "crate::hotkeys::Hotkeys::is_default")]
    pub hotkeys: crate::hotkeys::Hotkeys,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stopwatches: Vec<crate::stopwatch::Config>,
    #[serde(
        default = "crate::image_strip::defaults",
        skip_serializing_if = "crate::image_strip::is_default"
    )]
    pub image_strips: Vec<crate::image_strip::Config>,
}
fn is_true(value: &bool) -> bool {
    *value
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            widget_order: Vec::new(),
            widget_menu: crate::widget_menu::Config::default(),
            league: "Standard".into(),
            filters: Filters::default(),
            scale: 1.0,
            search_range: 10.,
            automatic_search: true,
            map_check: crate::inspection::MapConfig::default(),
            hotkeys: crate::hotkeys::Hotkeys::default(),
            stopwatches: Vec::new(),
            image_strips: crate::image_strip::defaults(),
        }
    }
}
pub fn path() -> std::path::PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join(".config")
        })
        .join("awakened-poe-trade-rs/config.json")
}
impl Settings {
    pub fn load() -> Result<Self, String> {
        let path = path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let settings: Self =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Invalid settings: {e}"))?;
        settings.validate()?;
        Ok(settings)
    }
    pub fn widget_keys(&self) -> Vec<crate::widget_order::Key> {
        std::iter::once(crate::widget_order::Key::Menu)
            .chain(
                self.stopwatches
                    .iter()
                    .map(|c| crate::widget_order::Key::Stopwatch(c.id))
                    .chain(
                        self.image_strips
                            .iter()
                            .map(|c| crate::widget_order::Key::Image(c.id)),
                    ),
            )
            .collect()
    }
    pub fn validate(&self) -> Result<(), String> {
        self.widget_menu.validate()?;
        let keys = self.widget_keys();
        let mut order_ids = std::collections::HashSet::new();
        if self
            .widget_order
            .iter()
            .any(|k| !keys.contains(k) || !order_ids.insert(k))
        {
            return Err("Ordre des widgets invalide ou dupliqué.".into());
        }
        if self.image_strips.len() > crate::image_strip::MAX_STRIPS {
            return Err("Au plus 16 bandes d’images.".into());
        }
        let mut image_ids = std::collections::HashSet::new();
        for c in &self.image_strips {
            c.validate()?;
            if !image_ids.insert(c.id) {
                return Err("Identifiant de bande d’images dupliqué.".into());
            }
        }
        self.map_check.validate()?;
        self.hotkeys.bindings()?;
        if self.stopwatches.len() > crate::stopwatch::MAX_INSTANCES {
            return Err("Au plus 32 chronomètres peuvent être configurés.".into());
        }
        let mut ids = std::collections::HashSet::new();
        for c in &self.stopwatches {
            c.validate()?;
            if !ids.insert(c.id) {
                return Err("Identifiant de chronomètre dupliqué.".into());
            }
        }
        if self
            .hotkeys
            .stopwatch_keys
            .iter()
            .any(|k| !ids.contains(&k.id))
        {
            return Err("Raccourci associé à un chronomètre absent.".into());
        }
        if !self.search_range.is_finite() || !(0. ..=100.).contains(&self.search_range) {
            return Err("La plage de recherche doit être comprise entre 0 et 100 %.".into());
        }
        if !crate::filters::PAYMENT_CURRENCIES
            .iter()
            .any(|(tag, _)| *tag == self.filters.bulk_currency)
        {
            return Err("Unsupported payment currency in settings".into());
        }
        if self.filters.default_currency.as_ref().is_some_and(|s| {
            s != "chaos_divine"
                && !crate::filters::PAYMENT_CURRENCIES
                    .iter()
                    .any(|(tag, _)| s == tag)
        }) {
            return Err("Devise de prix par défaut non prise en charge.".into());
        }
        if self.filters.min_item_level.is_some_and(|n| n > 100)
            || self.filters.min_links.is_some_and(|n| n > 6)
            || self.filters.minimum_stock.is_some_and(|n| n > 1_000_000)
        {
            return Err("Filter value exceeds the supported UI range".into());
        }
        if self.league.trim().is_empty() || self.league.len() > 100 {
            return Err("League must contain 1–100 characters".into());
        }
        if !self.scale.is_finite() || !(0.75..=2.5).contains(&self.scale) {
            return Err("Scale must be between 0.75 and 2.5".into());
        }
        Ok(())
    }
    pub fn save(&self) -> Result<(), String> {
        self.validate()?;
        let path = path();
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(tmp, path).map_err(|e| e.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_01_settings_keep_new_exchange_defaults() {
        let settings:Settings=serde_json::from_str(r#"{"league":"Covens","filters":{"online":true,"min_item_level":null,"min_links":null,"stats":[]},"scale":1}"#).unwrap();
        settings.validate().unwrap();
        assert_eq!(settings.filters.bulk_currency, "chaos");
        assert_eq!(settings.filters.minimum_stock, None);
        assert_eq!(settings.search_range, 10.);
        assert!(settings.filters.merchant_only);
        assert!(settings.filters.collapse_listings);
        assert!(settings.filters.item.is_none());
        assert_eq!(settings.map_check, crate::inspection::MapConfig::default());
        assert!(settings.hotkeys.is_default());
        assert!(settings.automatic_search);
        assert!(serde_json::to_value(&settings)
            .unwrap()
            .get("automatic_search")
            .is_none());
        assert!(serde_json::to_value(&settings)
            .unwrap()
            .get("hotkeys")
            .is_none());
    }
    #[test]
    fn invalid_settings_are_rejected_before_the_ui_can_silently_clamp_them() {
        let mut s = Settings::default();
        s.filters.minimum_stock = Some(1_000_001);
        assert!(s.validate().is_err());
        s.filters.minimum_stock = None;
        s.filters.bulk_currency = "unknown".into();
        assert!(s.validate().is_err());
        s.filters.bulk_currency = "chaos".into();
        s.scale = f64::NAN;
        assert!(s.validate().is_err());
        s.scale = 1.;
        s.search_range = 101.;
        assert!(s.validate().is_err());
    }
}

#[cfg(test)]
mod stopwatch_tests {
    use super::*;
    #[test]
    fn widget_order_is_optional_and_rejects_dangling_or_duplicate_keys() {
        use crate::widget_order::Key;
        let mut s = Settings::default();
        assert!(serde_json::to_value(&s)
            .unwrap()
            .get("widget_order")
            .is_none());
        s.stopwatches.push(crate::stopwatch::Config::default());
        s.widget_order = vec![Key::Image(1), Key::Stopwatch(1)];
        s.validate().unwrap();
        let roundtrip: Settings =
            serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
        assert_eq!(roundtrip.widget_order, s.widget_order);
        s.widget_order.push(Key::Image(1));
        assert!(s.validate().is_err());
        s.widget_order = vec![Key::Stopwatch(2)];
        assert!(s.validate().is_err());
    }
    #[test]
    fn timer_keys_require_valid_unique_instances_and_share_global_reservations() {
        let mut s = Settings::default();
        s.hotkeys
            .stopwatch_keys
            .push(crate::hotkeys::StopwatchKeys {
                id: 7,
                toggle: Some("Ctrl + J".into()),
                reset: None,
            });
        assert!(s.validate().is_err());
        s.stopwatches.push(crate::stopwatch::Config {
            id: 7,
            ..Default::default()
        });
        s.validate().unwrap();
        let bindings = s.hotkeys.bindings().unwrap();
        assert!(bindings
            .iter()
            .any(|(_, a)| *a == crate::hotkeys::Action::StopwatchToggle(7)));
        s.hotkeys.stopwatch_keys[0].toggle = Some("Ctrl + D".into());
        assert!(s.validate().is_err());
        s.hotkeys.stopwatch_keys[0].toggle = Some("Ctrl + Alt + C".into());
        assert!(s.validate().is_err());
        s.hotkeys.stopwatch_keys[0].toggle = None;
        s.stopwatches.push(s.stopwatches[0].clone());
        assert!(s.validate().is_err());
    }
}
