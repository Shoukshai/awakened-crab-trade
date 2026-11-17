use crate::filters::Filters;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub restore_clipboard: bool,
    #[serde(flatten)]
    pub legacy: std::collections::BTreeMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "is_true")]
    pub automatic_league: bool,
    pub league: String,
    pub filters: Filters,
    pub scale: f64,
    pub search_range: f64,
    #[serde(skip_serializing_if = "is_true")]
    pub automatic_search: bool,
    pub map_check: crate::inspection::MapConfig,
    #[serde(skip_serializing_if = "crate::hotkeys::Hotkeys::is_default")]
    pub hotkeys: crate::hotkeys::Hotkeys,
}
fn is_true(value: &bool) -> bool {
    *value
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            restore_clipboard: false,
            legacy: Default::default(),
            automatic_league: true,
            league: "Standard".into(),
            filters: Filters::default(),
            scale: 1.0,
            search_range: 10.,
            automatic_search: true,
            map_check: crate::inspection::MapConfig::default(),
            hotkeys: crate::hotkeys::Hotkeys::default(),
        }
    }
}
pub fn path() -> std::path::PathBuf {
    crate::paths::config_root().join("awakened-poe-trade-rs/config.json")
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
    pub fn validate(&self) -> Result<(), String> {
        self.checker_hotkeys().bindings()?;
        self.map_check.validate()?;
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
    /// Install only price-checker actions while preserving historical preferences.
    pub fn checker_hotkeys(&self) -> crate::hotkeys::Hotkeys {
        let mut h = crate::hotkeys::Hotkeys::disabled();
        h.price = self.hotkeys.price.clone();
        h.inspect = self.hotkeys.inspect.clone();
        h.interact = self.hotkeys.interact.clone();
        h.hide = self.hotkeys.hide.clone();
        h.settings = self.hotkeys.settings.clone();
        h
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_widget_data_is_preserved_without_installing_its_shortcuts() {
        let input = serde_json::json!({"widget_order":[{"type":"image","id":3}],"stash_scroll":true,"stopwatches":[{"id":999}],"hotkeys":{"delve":"Ctrl + D","similar":"Ctrl + K"}});
        let s: Settings = serde_json::from_value(input.clone()).unwrap();
        s.validate().unwrap();
        let output = serde_json::to_value(&s).unwrap();
        for field in ["widget_order", "stash_scroll", "stopwatches"] {
            assert_eq!(output[field], input[field]);
        }
        let bindings = s.checker_hotkeys().bindings().unwrap();
        assert!(bindings.iter().all(|(_, a)| matches!(
            a,
            crate::hotkeys::Action::Command(
                "copy" | "locked-copy" | "interact" | "hide" | "settings"
            )
        )));
        assert!(bindings
            .iter()
            .any(|(_, a)| *a == crate::hotkeys::Action::Command("locked-copy")));
    }
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
