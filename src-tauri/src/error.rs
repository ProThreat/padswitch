use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum PadSwitchError {
    #[error("Driver not installed: {0}")]
    DriverNotInstalled(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("HidHide error: {0}")]
    HidHide(String),

    #[error("ViGEmBus error: {0}")]
    ViGEm(String),

    #[error("Forwarding error: {0}")]
    Forwarding(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// Tauri requires commands to return Result<T, E> where E: Serialize
impl Serialize for PadSwitchError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PadSwitchError>;
