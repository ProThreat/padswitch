use crate::device::SlotAssignment;
use crate::error::{PadSwitchError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub assignments: Vec<SlotAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub auto_start: bool,
    pub start_minimized: bool,
    pub auto_forward_on_launch: bool,
    pub active_profile_id: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_start: false,
            start_minimized: false,
            auto_forward_on_launch: false,
            active_profile_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub settings: Settings,
    pub profiles: Vec<Profile>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            profiles: vec![],
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
