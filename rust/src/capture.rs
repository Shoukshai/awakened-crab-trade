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
    /// Optional tooltip rectangle in percentages of the client area.
    pub region: Option<[f64; 4]>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Clipboard,
            executable: "Boosteroid.exe".into(),
            region: None,
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
        if self.region.is_some_and(|r| {
            r.iter().any(|v| !v.is_finite() || *v < 0. || *v > 100.)
                || r[2] < 1.
                || r[3] < 1.
                || r[0] + r[2] > 100.
                || r[1] + r[3] > 100.
        }) {
            return Err("OCR area: x, y, width, height in percent, inside the window.".into());
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
            region: Some([58., 0., 38., 65.]),
        };
        settings.validate().unwrap();
        let decoded: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded, settings);
    }
    #[test]
    fn invalid_areas_and_clients_are_rejected_before_capture() {
        for region in [
            [0., 0., 0., 10.],
            [80., 0., 30., 40.],
            [0., f64::NAN, 20., 20.],
            [-1., 0., 20., 20.],
        ] {
            assert!(Settings {
                region: Some(region),
                ..Settings::default()
            }
            .validate()
            .is_err());
        }
        assert!(Settings {
            executable: "Boosteroid".into(),
            ..Settings::default()
        }
        .validate()
        .is_err());
    }
}
