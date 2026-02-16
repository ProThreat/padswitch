use crate::device::SlotAssignment;
use crate::error::{PadSwitchError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum RoutingMode {
    #[default]
    Minimal,
    Force,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub assignments: Vec<SlotAssignment>,
    #[serde(default)]
    pub routing_mode: RoutingMode,
}

/// A rule that maps a game executable to a preset profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRule {
    pub id: String,
    /// Executable filename to match (e.g. "RocketLeague.exe"). Case-insensitive.
    pub exe_name: String,
    /// Which profile to activate when this game is running.
    pub profile_id: String,
    /// Whether this rule is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub auto_start: bool,
    pub start_minimized: bool,
    pub auto_forward_on_launch: bool,
    /// Whether the process watcher is enabled (auto-switch presets on game launch).
    #[serde(default)]
    pub auto_switch: bool,
    pub active_profile_id: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_start: false,
            start_minimized: false,
            auto_forward_on_launch: false,
            auto_switch: false,
            active_profile_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub settings: Settings,
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub game_rules: Vec<GameRule>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            profiles: vec![],
            game_rules: vec![],
        }
    }
}

impl AppConfig {
    fn config_path() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| PadSwitchError::Config("Cannot find config directory".into()))?
            .join("padswitch");
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        let data = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&data)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data)?;
        Ok(())
    }
}
