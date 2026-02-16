use crate::device::{DriverStatus, GamepadState, PhysicalDevice};
use crate::error::{PadSwitchError, Result};
use crate::platform::{DeviceEnumerator, DeviceHider, VirtualControllerManager};

/// Linux stub -- future support via uinput / evdev.
pub struct LinuxPlatform;

impl LinuxPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceEnumerator for LinuxPlatform {
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>> {
        Ok(vec![])
    }

    fn check_drivers(&self) -> Result<DriverStatus> {
        Ok(DriverStatus::default())
    }
}

impl DeviceHider for LinuxPlatform {
    fn hide_device(&self, _instance_path: &str) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }

    fn unhide_device(&self, _instance_path: &str) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }

    fn whitelist_self(&self) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }

    fn disable_device(&self, _instance_path: &str) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }

    fn enable_device(&self, _instance_path: &str) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }

    fn deactivate_hiding(&self) -> Result<()> {
        Ok(()) // No hiding driver on Linux
    }
}

impl VirtualControllerManager for LinuxPlatform {
    fn create_virtual_controller(&self) -> Result<u32> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }

    fn destroy_virtual_controller(&self, _index: u32) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }

    fn read_gamepad_state(&self, _instance_path: &str) -> Result<GamepadState> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }

    fn write_virtual_state(&self, _index: u32, _state: &GamepadState) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported("Linux".into()))
    }
}
