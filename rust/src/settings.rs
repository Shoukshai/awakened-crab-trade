use crate::filters::Filters;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub capture: crate::capture::Settings,
    pub theme: crate::appearance::Theme,
    #[serde(default = "crate::i18n::Language::legacy_default")]
    pub language: crate::i18n::Language,
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
            capture: crate::capture::Settings::default(),
            theme: crate::appearance::Theme::default(),
            language: crate::i18n::Language::En,
            restore_clipboard: false,
            legacy: Default::default(),
            automatic_league: true,
            league: "Standard".into(),
            filters: Filters::default(),
            scale: 1.0,
            search_range: 10.,
            map_check: crate::inspection::MapConfig::default(),
            hotkeys: crate::hotkeys::Hotkeys::default(),
        }
    }
}
pub fn path() -> std::path::PathBuf {
    crate::paths::config_root().join("awakened-crab-trade/config.json")
}
impl Settings {
    pub fn load() -> Result<Self, String> {
        Self::load_from_root(&crate::paths::config_root())
    }
    fn load_from_root(root: &std::path::Path) -> Result<Self, String> {
        let path = root.join("awakened-crab-trade/config.json");
        if !path.exists() {
            let legacy = crate::compat::legacy_settings(root);
            if !legacy.exists() {
                return Ok(Self::default());
            }
            let mut imported: Self =
                serde_json::from_slice(&std::fs::read(legacy).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Invalid settings: {e}"))?;
            imported.legacy.remove("automatic_search");
            imported.validate()?;
            crate::compat::publish_import(
                &path,
                &serde_json::to_vec_pretty(&imported).map_err(|e| e.to_string())?,
            )?;
        }
        let mut settings: Self =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Invalid settings: {e}"))?;
        settings.legacy.remove("automatic_search");
        settings.validate()?;
        Ok(settings)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.capture.validate()?;
        let bindings = self.checker_hotkeys().bindings()?;
        if self.capture.mode == crate::capture::Mode::Ocr
            && !bindings
                .iter()
                .any(|(key, action)| key.alt && action.is_copy())
        {
            return Err(
                "OCR capture needs a price shortcut containing Alt, for example Ctrl + Alt + D."
                    .into(),
            );
        }
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
        let mut current = self.clone();
        current.legacy.remove("automatic_search");
        let path = path();
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(&current).map_err(|e| e.to_string())?,
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
    fn legacy_profiles_are_imported_once_and_automatic_search_is_discarded() {
        let root = std::env::temp_dir().join(format!(
            "act-profile-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let old = crate::compat::legacy_settings(&root);
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        let bytes=br#"{"league":"Covens","automatic_search":true,"restore_clipboard":true,"custom_ui":{"kept":42}}"#;
        std::fs::write(&old, bytes).unwrap();
        let imported = Settings::load_from_root(&root).unwrap();
        assert_eq!(imported.language, crate::i18n::Language::Fr);
        assert_eq!(imported.league, "Covens");
        assert!(imported.restore_clipboard);
        assert_eq!(imported.legacy["custom_ui"]["kept"], 42);
        let new = root.join("awakened-crab-trade/config.json");
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&new).unwrap()).unwrap();
        assert!(saved.get("automatic_search").is_none());
        assert_eq!(std::fs::read(&old).unwrap(), bytes);
        let mut changed = saved;
        changed["league"] = serde_json::json!("Standard");
        changed["language"] = serde_json::json!("en");
        std::fs::write(&new, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert_eq!(
            Settings::load_from_root(&root).unwrap().language,
            crate::i18n::Language::En
        );
        assert_eq!(Settings::load_from_root(&root).unwrap().league, "Standard");
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn old_profiles_default_to_tide_and_all_theme_choices_persist_without_changing_filters() {
        let old: Settings =
            serde_json::from_value(serde_json::json!({"league":"Covens","custom":42})).unwrap();
        assert_eq!(old.theme, crate::appearance::Theme::Tide);
        let original_filters = serde_json::to_value(&old.filters).unwrap();
        for theme in crate::appearance::Theme::ALL {
            let mut settings = old.clone();
            settings.theme = theme;
            let encoded = serde_json::to_value(&settings).unwrap();
            assert_eq!(encoded["theme"], theme.id());
            let decoded: Settings = serde_json::from_value(encoded).unwrap();
            assert_eq!(decoded.theme, theme);
            assert_eq!(
                serde_json::to_value(decoded.filters).unwrap(),
                original_filters
            );
            assert_eq!(decoded.legacy["custom"], 42);
        }
        assert!(
            serde_json::from_value::<Settings>(serde_json::json!({"theme":"unknown"})).is_err()
        );
    }
    #[test]
    fn fresh_profiles_start_in_english() {
        assert_eq!(Settings::default().language, crate::i18n::Language::En);
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
