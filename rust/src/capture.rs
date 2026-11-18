//! Capture preferences are separate from keyboard bindings: OCR never copies.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Clipboard,
    Ocr,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub mode: Mode,
    /// Executable basename or absolute path chosen by the user, never a title.
    pub executable: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: crate::edition::capture_mode(),
            executable: "Boosteroid.exe".into(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self.executable.trim().is_empty()
            || self.executable.len() > 1024
            || !self.executable.to_ascii_lowercase().ends_with(".exe")
        {
            return Err("Choose the cloud client's .exe filename or full path.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn older_profiles_keep_clipboard_and_cloud_configuration_round_trips() {
        let old: crate::settings::Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.capture.mode, Mode::Clipboard);
        let settings = Settings {
            mode: Mode::Ocr,
            executable: r"C:\Program Files\Boosteroid\Boosteroid.exe".into(),
        };
        settings.validate().unwrap();
        let decoded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded, settings);
    }
    #[test]
    fn invalid_clients_are_rejected_before_capture() {
        assert!(Settings {
            executable: "Boosteroid".into(),
            ..Settings::default()
        }
        .validate()
        .is_err());
    }
}
