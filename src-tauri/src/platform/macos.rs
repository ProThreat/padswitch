use crate::device::{DeviceType, DriverStatus, GamepadState, PhysicalDevice};
use crate::error::{PadSwitchError, Result};
use crate::platform::ControllerManager;

/// macOS stub — returns mock data for development/testing.
/// Real macOS support (IOKit/GameController framework) is a future goal.
pub struct MacOSControllerManager;

impl MacOSControllerManager {
    pub fn new() -> Self {
        Self
    }
}

impl ControllerManager for MacOSControllerManager {
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>> {
        // Return mock devices for UI development
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
            },
        ])
    }

    fn check_drivers(&self) -> Result<DriverStatus> {
        // On macOS, report drivers as "installed" so the UI doesn't show warnings during dev
        Ok(DriverStatus {
            hidhide_installed: true,
            vigembus_installed: true,
            hidhide_version: Some("(mock — macOS dev mode)".into()),
            vigembus_version: Some("(mock — macOS dev mode)".into()),
        })
    }

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
