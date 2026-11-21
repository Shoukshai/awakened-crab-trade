//! Persisted visual choices. No filter or Trade behavior depends on a theme.
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Tide,
    Relic,
    Terminal,
    Frost,
    Split,
    Focus,
    Blueprint,
    Editorial,
    Compact,
    Merchant,
    Awakened,
}
impl Theme {
    pub const ALL: [Self; 11] = [
        Self::Tide,
        Self::Relic,
        Self::Terminal,
        Self::Frost,
        Self::Split,
        Self::Focus,
        Self::Blueprint,
        Self::Editorial,
        Self::Compact,
        Self::Merchant,
        Self::Awakened,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Tide => "tide",
            Self::Relic => "relic",
            Self::Terminal => "terminal",
            Self::Frost => "frost",
            Self::Split => "split",
            Self::Focus => "focus",
            Self::Blueprint => "blueprint",
            Self::Editorial => "editorial",
            Self::Compact => "compact",
            Self::Merchant => "merchant",
            Self::Awakened => "awakened",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Tide => "Tide",
            Self::Relic => "Relic",
            Self::Terminal => "Terminal",
            Self::Frost => "Frost",
            Self::Split => "Split",
            Self::Focus => "Focus",
            Self::Blueprint => "Blueprint",
            Self::Editorial => "Editorial",
            Self::Compact => "Compact",
            Self::Merchant => "Merchant",
            Self::Awakened => "Awakened Classic",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.id() == id)
    }
    pub fn width(self) -> i32 {
        match self {
            Self::Split => 640,
            Self::Editorial | Self::Merchant => 540,
            Self::Compact => 400,
            Self::Awakened => 460,
            _ => 480,
        }
    }
    pub fn font_size(self) -> u32 {
        match self {
            Self::Terminal | Self::Compact => 12,
            _ => 14,
        }
    }
    pub fn spacing(self) -> i32 {
        match self {
            Self::Terminal | Self::Compact | Self::Awakened => 8,
            Self::Editorial => 16,
            _ => 12,
        }
    }
    pub fn padding(self) -> u32 {
        match self {
            Self::Terminal | Self::Compact => 12,
            Self::Editorial => 24,
            Self::Awakened => 0,
            _ => 16,
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Tide => "Marine et cyan, lignes sobres.",
            Self::Relic => "Encre et cuivre, typographie PoE.",
            Self::Terminal => "Grille compacte et police monospace.",
            Self::Frost => "Surface claire et contraste bleu.",
            Self::Split => "Filtres et offres côte à côte.",
            Self::Focus => "Onglets Filtres et Offres.",
            Self::Blueprint => "Inspecteur carré, sections numérotées.",
            Self::Editorial => "Titres serif, lignes ouvertes et aérées.",
            Self::Compact => "Panneau étroit, recherche fixée en bas.",
            Self::Merchant => "Papier chaud, prix en première colonne.",
            Self::Awakened => {
                "Disposition classique d’Awakened PoE Trade, gris ardoise et lignes compactes."
            }
        }
    }
    pub fn colors(self) -> [&'static str; 9] {
        // Background, surface, hover, text, muted, border, accent, selection, on-accent.
        match self {
            Self::Tide => [
                "#0d1722", "#142330", "#1b2d3a", "#e2eaf0", "#a0b1bf", "#304452", "#68d9e7",
                "#15333e", "#07181c",
            ],
            Self::Relic => [
                "#151411", "#201e19", "#29261f", "#e9e2d4", "#b1a895", "#474238", "#e2a17a",
                "#2b211a", "#22170f",
            ],
            Self::Terminal => [
                "#101416", "#171e22", "#202a2e", "#dce4e7", "#a4b3b9", "#354047", "#7ac8e3",
                "#182c35", "#101a1e",
            ],
            Self::Frost => [
                "#edf2f3", "#e2e9ec", "#d5e1e5", "#172a34", "#526672", "#b3c1c8", "#086f85",
                "#d4e6ea", "#ffffff",
            ],
            Self::Split => [
                "#121d27", "#1b2a36", "#243847", "#e7edf2", "#a5b7c7", "#374e5e", "#7bbbea",
                "#203848", "#0e2330",
            ],
            Self::Focus => [
                "#131927", "#1c2639", "#28344b", "#e9edf7", "#aebbd3", "#3c4962", "#93afff",
                "#253553", "#111d38",
            ],
            Self::Blueprint => [
                "#15202c", "#1d2c3c", "#293d50", "#e6edf7", "#afbed0", "#42566b", "#81b6ed",
                "#223c56", "#102338",
            ],
            Self::Editorial => [
                "#1e1d1c", "#282624", "#33302b", "#f0e9dc", "#bcb2a1", "#514b44", "#e4ac87",
                "#31281f", "#26190e",
            ],
            Self::Compact => [
                "#101b20", "#192930", "#243940", "#e4eeef", "#a4b8be", "#344d56", "#64d6e1",
                "#18363d", "#09272c",
            ],
            Self::Awakened => [
                "#2d3748", "#1a202c", "#4a5568", "#e2e8f0", "#a0aec0", "#4a5568", "#cbd5e0",
                "#4a5568", "#1a202c",
            ],
            Self::Merchant => [
                "#eee8dd", "#e5dccd", "#ded1bc", "#332b23", "#6e6051", "#bfb09b", "#9d4f2b",
                "#e8d7c3", "#fff9ee",
            ],
        }
    }
    pub fn light(self) -> bool {
        matches!(self, Self::Frost | Self::Merchant)
    }
    pub fn definitions(self) -> String {
        let mut css = String::new();
        for (name, value) in [
            "bg",
            "surface",
            "hover",
            "fg",
            "muted",
            "line",
            "accent",
            "selected",
            "on_accent",
        ]
        .into_iter()
        .zip(self.colors())
        {
            css.push_str(&format!("@define-color act_{name} {value};\n"));
        }
        for (name, value) in [
            ("track", if self.light() { "#b8c2c9" } else { "#262b33" }),
            ("normal", if self.light() { "#172a34" } else { "#e2e5e8" }),
            ("magic", if self.light() { "#3e4d9d" } else { "#a9b7ff" }),
            ("rare", if self.light() { "#775700" } else { "#fff185" }),
            ("unique", if self.light() { "#9b4520" } else { "#f1aa77" }),
            ("gem", if self.light() { "#17685c" } else { "#77d5c2" }),
            (
                "corrupted",
                if self.light() { "#a33428" } else { "#f0a69a" },
            ),
        ] {
            css.push_str(&format!("@define-color act_{name} {value};\n"));
        }
        css
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_style_has_a_stable_round_trip_identity_and_layout_width() {
        let mut ids = std::collections::HashSet::new();
        for theme in Theme::ALL {
            assert!(ids.insert(theme.id()));
            assert_eq!(
                serde_json::from_str::<Theme>(&serde_json::to_string(&theme).unwrap()).unwrap(),
                theme
            );
            assert_eq!(Theme::from_id(theme.id()), Some(theme));
            assert!((360..=640).contains(&theme.width()));
            for color in theme.colors() {
                assert_eq!(color.len(), 7);
                assert!(u32::from_str_radix(&color[1..], 16).is_ok());
            }
        }
        assert_eq!(Theme::from_id("unknown"), None);
    }
}
