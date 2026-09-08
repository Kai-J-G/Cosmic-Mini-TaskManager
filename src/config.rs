//! Persisted settings, stored through `cosmic-config`.

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "io.github.kai_j_g.CosmicMiniTaskManager";
pub const CONFIG_VERSION: u64 = 1;

/// Whether the popup follows the desktop theme or is pinned to one appearance.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Default)]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemePreference {
    /// The theme to apply for this preference.
    ///
    /// `Dark` and `Light` set `prefer_dark` so that the shell's own
    /// light/dark switch does not override an explicit choice; `System`
    /// leaves it unset so the theme keeps following the desktop.
    pub fn theme(self) -> cosmic::Theme {
        let mut theme = match self {
            Self::System => cosmic::theme::system_preference(),
            Self::Dark => {
                let mut t = cosmic::theme::system_dark();
                t.theme_type.prefer_dark(Some(true));
                t
            }
            Self::Light => {
                let mut t = cosmic::theme::system_light();
                t.theme_type.prefer_dark(Some(false));
                t
            }
        };
        // Applet popups blur whatever is behind them.
        theme.transparent = true;
        theme
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, CosmicConfigEntry)]
#[version = 1]
pub struct MiniTaskManagerConfig {
    pub refresh_interval_secs: u64,
    pub theme_pref: ThemePreference,
    pub warn_unresponsive_in_panel: bool,
}

impl Default for MiniTaskManagerConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 2,
            theme_pref: ThemePreference::System,
            warn_unresponsive_in_panel: true,
        }
    }
}
