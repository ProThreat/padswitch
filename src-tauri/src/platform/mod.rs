use crate::device::{DriverStatus, GamepadState, PhysicalDevice};
use crate::error::Result;

/// Platform-specific controller management operations.
/// Implementations live in windows.rs / macos.rs / linux.rs.
pub trait ControllerManager: Send + Sync {
    /// Enumerate currently connected physical game controllers.
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>>;

    /// Check if required drivers (HidHide, ViGEmBus) are installed.
    fn check_drivers(&self) -> Result<DriverStatus>;

    /// Hide a physical device from other applications (via HidHide on Windows).
    fn hide_device(&self, instance_path: &str) -> Result<()>;

    /// Unhide a previously hidden device.
    fn unhide_device(&self, instance_path: &str) -> Result<()>;

    /// Whitelist this application so it can still see hidden devices.
    fn whitelist_self(&self) -> Result<()>;

    /// Create a virtual XInput controller at the next available slot.
    fn create_virtual_controller(&self) -> Result<u32>;

    /// Destroy a virtual controller by its handle/index.
    fn destroy_virtual_controller(&self, index: u32) -> Result<()>;

    /// Read the current state of a physical device.
    fn read_gamepad_state(&self, instance_path: &str) -> Result<GamepadState>;

    /// Write gamepad state to a virtual controller.
    fn write_virtual_state(&self, index: u32, state: &GamepadState) -> Result<()>;
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;

/// Create the platform-appropriate ControllerManager.
pub fn create_manager() -> Box<dyn ControllerManager> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsControllerManager::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOSControllerManager::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxControllerManager::new())
    }
}
