use crate::device::{DriverStatus, GamepadState, PhysicalDevice};
use crate::error::Result;
use std::sync::Arc;

/// Enumerate connected physical game controllers and check driver status.
pub trait DeviceEnumerator: Send + Sync {
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>>;
    fn check_drivers(&self) -> Result<DriverStatus>;
}

/// Hide/unhide physical devices from other applications (HidHide on Windows).
/// Also disable/enable devices via OS-level APIs (SetupDi on Windows).
pub trait DeviceHider: Send + Sync {
    /// Hide a device via HidHide (force mode). Requires HidHide driver.
    fn hide_device(&self, instance_path: &str) -> Result<()>;
    /// Unhide a device via HidHide (force mode).
    fn unhide_device(&self, instance_path: &str) -> Result<()>;
    /// Add our process to HidHide whitelist so we can still read hidden devices.
    fn whitelist_self(&self) -> Result<()>;
    /// Disable a device via OS APIs (minimal mode). May require admin.
    fn disable_device(&self, instance_path: &str) -> Result<()>;
    /// Enable a device via OS APIs (minimal mode).
    fn enable_device(&self, instance_path: &str) -> Result<()>;
    /// Deactivate the hiding driver globally (HidHide on Windows). No-op on other platforms.
    fn deactivate_hiding(&self) -> Result<()>;
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

/// Check whether the current process is running with admin/elevated privileges.
/// Minimal mode (SetupDi disable/enable) requires elevation on Windows.
pub fn is_elevated() -> bool {
    #[cfg(target_os = "windows")]
    {
        unsafe { windows::Win32::UI::Shell::IsUserAnAdmin().as_bool() }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // On macOS/Linux, elevation isn't required for the current feature set
        true
    }
}
