use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceType {
    XInput,
    DirectInput,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalDevice {
    /// Unique ID for this device within PadSwitch
    pub id: String,
    /// Human-readable name (e.g., "Xbox Wireless Controller")
    pub name: String,
    /// Device instance path (Windows) or IOKit path (macOS)
    pub instance_path: String,
    /// Type of input device
    pub device_type: DeviceType,
    /// Whether the device is currently hidden from games
    pub hidden: bool,
    /// Whether the device is currently connected
    pub connected: bool,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
}

impl PhysicalDevice {
    pub fn new(name: String, instance_path: String, device_type: DeviceType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            instance_path,
            device_type,
            hidden: false,
            connected: true,
            vendor_id: 0,
            product_id: 0,
        }
    }
}

/// Represents the user's desired mapping: physical device → virtual XInput slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotAssignment {
    /// ID of the physical device
    pub device_id: String,
    /// Target XInput slot (0-3)
    pub slot: u8,
    /// Whether this assignment is enabled
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverStatus {
    pub hidhide_installed: bool,
    pub vigembus_installed: bool,
    pub hidhide_version: Option<String>,
    pub vigembus_version: Option<String>,
}

impl Default for DriverStatus {
    fn default() -> Self {
        Self {
            hidhide_installed: false,
            vigembus_installed: false,
            hidhide_version: None,
            vigembus_version: None,
        }
    }
}

/// XInput gamepad state for forwarding
#[derive(Debug, Clone, Default)]
pub struct GamepadState {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    pub thumb_rx: i16,
    pub thumb_ry: i16,
}
