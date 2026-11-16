use crate::filters::Filters;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub league: String,
    pub filters: Filters,
    pub scale: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            league: "Standard".into(),
            filters: Filters::default(),
            scale: 1.0,
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
