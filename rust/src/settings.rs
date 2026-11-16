use crate::filters::Filters;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub league: String,
    pub filters: Filters,
    pub scale: f64,
    pub search_range: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            league: "Standard".into(),
            filters: Filters::default(),
            scale: 1.0,
            search_range: 10.,
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
    pub fn validate(&self) -> Result<(), String> {
        if !self.search_range.is_finite() || !(0. ..=100.).contains(&self.search_range) {
            return Err("La plage de recherche doit être comprise entre 0 et 100 %.".into());
        }
        if !crate::filters::PAYMENT_CURRENCIES
            .iter()
            .any(|(tag, _)| *tag == self.filters.bulk_currency)
        {
            return Err("Unsupported payment currency in settings".into());
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
