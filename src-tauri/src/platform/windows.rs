use crate::device::{DriverStatus, GamepadState, PhysicalDevice};
use crate::error::{PadSwitchError, Result};
use crate::hidhide::imp::HidHide;
use crate::platform::{DeviceEnumerator, DeviceHider, VirtualControllerManager};
use crate::vigem;
use std::sync::Mutex;

/// Windows implementation using XInput + HidHide + ViGEmBus.
pub struct WindowsPlatform {
    xinput: Mutex<Option<rusty_xinput::XInputHandle>>,
}

impl WindowsPlatform {
    pub fn new() -> Self {
        let handle = rusty_xinput::XInputHandle::load_default().ok();
        Self {
            xinput: Mutex::new(handle),
        }
    }
}

/// Parse an XInput slot number from an instance path like "XINPUT\SLOT2".
fn parse_xinput_slot(instance_path: &str) -> Result<u32> {
    let upper = instance_path.to_uppercase();
    if let Some(rest) = upper.strip_prefix("XINPUT\\SLOT") {
        rest.parse::<u32>().map_err(|_| {
            PadSwitchError::Platform(format!("Invalid XInput slot in path: {}", instance_path))
        })
    } else {
        Err(PadSwitchError::Platform(format!(
            "Not an XInput instance path: {}",
            instance_path
        )))
    }
}

impl DeviceEnumerator for WindowsPlatform {
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let guard = self.xinput.lock().unwrap();
        let handle = match guard.as_ref() {
            Some(h) => h,
            None => return Ok(vec![]),
        };

        let mut devices = Vec::new();
        for slot in 0..4u32 {
            if handle.get_state(slot).is_ok() {
                devices.push(PhysicalDevice::from_xinput_slot(slot));
            }
        }
        Ok(devices)
    }

    fn check_drivers(&self) -> Result<DriverStatus> {
        Ok(DriverStatus {
            hidhide_installed: HidHide::is_installed(),
            vigembus_installed: vigem::imp::is_installed(),
            hidhide_version: None,
            vigembus_version: None,
        })
    }
}

impl DeviceHider for WindowsPlatform {
    fn hide_device(&self, instance_path: &str) -> Result<()> {
        let hh = HidHide::open()?;
        hh.add_to_blacklist(instance_path)
    }

    fn unhide_device(&self, instance_path: &str) -> Result<()> {
        let hh = HidHide::open()?;
        hh.remove_from_blacklist(instance_path)
    }

    fn whitelist_self(&self) -> Result<()> {
        let exe = std::env::current_exe()
            .map_err(|e| PadSwitchError::Platform(format!("Failed to get current exe: {}", e)))?;
        let exe_str = exe.to_string_lossy().to_string();
        let hh = HidHide::open()?;
        hh.add_to_whitelist(&exe_str)
    }
}

impl VirtualControllerManager for WindowsPlatform {
    fn create_virtual_controller(&self) -> Result<u32> {
        // Virtual controllers are created inside the input loop thread
        // to avoid lifetime issues. This is a no-op at the platform level.
        Ok(0)
    }

    fn destroy_virtual_controller(&self, _index: u32) -> Result<()> {
        // Virtual controllers are destroyed when the input loop thread ends.
        Ok(())
    }

    fn read_gamepad_state(&self, instance_path: &str) -> Result<GamepadState> {
        let slot = parse_xinput_slot(instance_path)?;
        let guard = self.xinput.lock().unwrap();
        let handle = guard.as_ref().ok_or_else(|| {
            PadSwitchError::Platform("XInput not loaded".into())
        })?;

        let state = handle.get_state(slot).map_err(|_| {
            PadSwitchError::Platform(format!("Failed to read XInput slot {}", slot))
        })?;

        Ok(GamepadState {
            buttons: state.raw.Gamepad.wButtons,
            left_trigger: state.raw.Gamepad.bLeftTrigger,
            right_trigger: state.raw.Gamepad.bRightTrigger,
            thumb_lx: state.raw.Gamepad.sThumbLX,
            thumb_ly: state.raw.Gamepad.sThumbLY,
            thumb_rx: state.raw.Gamepad.sThumbRX,
            thumb_ry: state.raw.Gamepad.sThumbRY,
        })
    }

    fn write_virtual_state(&self, _index: u32, _state: &GamepadState) -> Result<()> {
        // Writing happens inside the input loop thread directly via vigem_client.
        Ok(())
    }
}
