use crate::device::{DeviceType, DriverStatus, GamepadState, PhysicalDevice};
use crate::error::{PadSwitchError, Result};
use crate::platform::{DeviceEnumerator, DeviceHider, VirtualControllerManager};

/// macOS stub -- returns mock data for development/testing.
pub struct MacOSPlatform;

impl MacOSPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceEnumerator for MacOSPlatform {
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>> {
        Ok(vec![
            PhysicalDevice {
                id: "mock-wooting-60he".into(),
                name: "Wooting 60HE (Gamepad Mode)".into(),
                instance_path: "/mock/wooting".into(),
                device_type: DeviceType::XInput,
                hidden: false,
                connected: true,
                vendor_id: 0x31E3,
                product_id: 0x1100,
                xinput_slot: Some(0),
            },
            PhysicalDevice {
                id: "mock-xbox-controller".into(),
                name: "Xbox Wireless Controller".into(),
                instance_path: "/mock/xbox".into(),
                device_type: DeviceType::XInput,
                hidden: false,
                connected: true,
                vendor_id: 0x045E,
                product_id: 0x0B12,
                xinput_slot: Some(1),
            },
            PhysicalDevice {
                id: "mock-ps5-dualsense".into(),
                name: "DualSense Wireless Controller".into(),
                instance_path: "/mock/dualsense".into(),
                device_type: DeviceType::DirectInput,
                hidden: false,
                connected: true,
                vendor_id: 0x054C,
                product_id: 0x0CE6,
                xinput_slot: None,
            },
        ])
    }

    fn check_drivers(&self) -> Result<DriverStatus> {
        Ok(DriverStatus {
            hidhide_installed: true,
            vigembus_installed: true,
            hidhide_version: Some("(mock — macOS dev mode)".into()),
            vigembus_version: Some("(mock — macOS dev mode)".into()),
        })
    }
}

impl DeviceHider for MacOSPlatform {
    fn hide_device(&self, instance_path: &str) -> Result<()> {
        log::info!("[macOS stub] hide_device: {}", instance_path);
        Ok(())
    }

    fn unhide_device(&self, instance_path: &str) -> Result<()> {
        log::info!("[macOS stub] unhide_device: {}", instance_path);
        Ok(())
    }

    fn whitelist_self(&self) -> Result<()> {
        log::info!("[macOS stub] whitelist_self");
        Ok(())
    }

    fn disable_device(&self, instance_path: &str) -> Result<()> {
        log::info!("[macOS stub] disable_device: {}", instance_path);
        Ok(())
    }

    fn enable_device(&self, instance_path: &str) -> Result<()> {
        log::info!("[macOS stub] enable_device: {}", instance_path);
        Ok(())
    }

    fn deactivate_hiding(&self) -> Result<()> {
        log::info!("[macOS stub] deactivate_hiding");
        Ok(())
    }
}

impl VirtualControllerManager for MacOSPlatform {
    fn create_virtual_controller(&self) -> Result<u32> {
        Err(PadSwitchError::PlatformNotSupported(
            "Virtual controllers not available on macOS".into(),
        ))
    }

    fn destroy_virtual_controller(&self, _index: u32) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported(
            "Virtual controllers not available on macOS".into(),
        ))
    }

    fn read_gamepad_state(&self, _instance_path: &str) -> Result<GamepadState> {
        Ok(GamepadState::default())
    }

    fn write_virtual_state(&self, _index: u32, _state: &GamepadState) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported(
            "Virtual controllers not available on macOS".into(),
        ))
    }
}
