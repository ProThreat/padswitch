use crate::device::{DriverStatus, GamepadState, PhysicalDevice};
use crate::error::Result;
use std::sync::Arc;

/// Enumerate connected physical game controllers and check driver status.
pub trait DeviceEnumerator: Send + Sync {
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>>;
    fn check_drivers(&self) -> Result<DriverStatus>;
}

/// Hide/unhide physical devices from other applications (HidHide on Windows).
pub trait DeviceHider: Send + Sync {
    fn hide_device(&self, instance_path: &str) -> Result<()>;
    fn unhide_device(&self, instance_path: &str) -> Result<()>;
    fn whitelist_self(&self) -> Result<()>;
}

/// Create/destroy virtual XInput controllers and forward gamepad state.
pub trait VirtualControllerManager: Send + Sync {
    fn create_virtual_controller(&self) -> Result<u32>;
    fn destroy_virtual_controller(&self, index: u32) -> Result<()>;
    fn read_gamepad_state(&self, instance_path: &str) -> Result<GamepadState>;
    fn write_virtual_state(&self, index: u32, state: &GamepadState) -> Result<()>;
}

/// Combined trait for full platform support.
pub trait PlatformServices: DeviceEnumerator + DeviceHider + VirtualControllerManager {}

// Blanket impl: anything implementing all three sub-traits is a PlatformServices.
impl<T: DeviceEnumerator + DeviceHider + VirtualControllerManager> PlatformServices for T {}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;

/// Create the platform-appropriate service provider (singleton-friendly).
pub fn create_platform() -> Arc<dyn PlatformServices> {
    #[cfg(target_os = "windows")]
    {
        Arc::new(windows::WindowsPlatform::new())
    }
    #[cfg(target_os = "macos")]
    {
        Arc::new(macos::MacOSPlatform::new())
    }
    #[cfg(target_os = "linux")]
    {
        Arc::new(linux::LinuxPlatform::new())
    }
}
