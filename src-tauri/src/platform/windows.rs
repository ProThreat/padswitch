use crate::device::{DeviceType, DriverStatus, GamepadState, PhysicalDevice};
use crate::error::{PadSwitchError, Result};
use crate::hidhide::imp::HidHide;
use crate::platform::{DeviceEnumerator, DeviceHider, VirtualControllerManager};
use crate::setupdi::imp as setupdi;
use crate::vigem;
use std::sync::Mutex;

/// Windows implementation using SetupAPI + XInput + HidHide + ViGEmBus.
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

    /// Get connected XInput slot numbers (0-3).
    fn connected_xinput_slots(&self) -> Vec<u32> {
        let guard = self.xinput.lock().unwrap();
        let Some(handle) = guard.as_ref() else {
            return vec![];
        };
        (0..4u32).filter(|&s| handle.get_state(s).is_ok()).collect()
    }
}

impl DeviceEnumerator for WindowsPlatform {
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let connected_slots = self.connected_xinput_slots();

        // Try real device enumeration via SetupAPI
        let real_devices = setupdi::enumerate_game_controllers().unwrap_or_else(|e| {
            log::warn!("SetupAPI enumeration failed, falling back to XInput-only: {}", e);
            vec![]
        });

        if !real_devices.is_empty() {
            let mut devices = Vec::new();

            // Separate XInput-compatible devices from DirectInput-only devices.
            // Only XInput devices occupy XInput slots 0-3; DirectInput devices
            // don't get a slot number assigned.
            let mut slot_iter = connected_slots.iter().copied();

            for dev in &real_devices {
                let xinput_slot = if dev.is_xinput {
                    let slot = slot_iter.next();
                    if slot.is_some() {
                        log::debug!(
                            "XInput slot {:?} -> {} ({})",
                            slot,
                            dev.name,
                            dev.instance_path
                        );
                    }
                    slot
                } else {
                    log::debug!(
                        "DirectInput device (no XInput slot): {} ({})",
                        dev.name,
                        dev.instance_path
                    );
                    None
                };

                devices.push(PhysicalDevice {
                    id: setupdi::stable_device_id(&dev.instance_path),
                    name: dev.name.clone(),
                    instance_path: dev.instance_path.clone(),
                    device_type: if dev.is_xinput {
                        DeviceType::XInput
                    } else {
                        DeviceType::DirectInput
                    },
                    hidden: false,
                    connected: true,
                    vendor_id: dev.vendor_id,
                    product_id: dev.product_id,
                    xinput_slot,
                });
            }

            // If there are leftover connected XInput slots that didn't match
            // any SetupAPI device, create fallback entries.
            for slot in slot_iter {
                log::debug!("Unmatched XInput slot {} — creating fallback device", slot);
                devices.push(PhysicalDevice::from_xinput_slot(slot));
            }

            return Ok(devices);
        }

        // Fallback: XInput-only enumeration (no SetupAPI devices found)
        let devices = connected_slots
            .iter()
            .map(|&slot| PhysicalDevice::from_xinput_slot(slot))
            .collect();
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

    fn disable_device(&self, instance_path: &str) -> Result<()> {
        setupdi::disable_device(instance_path)
    }

    fn enable_device(&self, instance_path: &str) -> Result<()> {
        setupdi::enable_device(instance_path)
    }

    fn deactivate_hiding(&self) -> Result<()> {
        let hh = HidHide::open()?;
        hh.set_active(false)
    }
}

impl VirtualControllerManager for WindowsPlatform {
    fn create_virtual_controller(&self) -> Result<u32> {
        Ok(0)
    }

    fn destroy_virtual_controller(&self, _index: u32) -> Result<()> {
        Ok(())
    }

    fn read_gamepad_state(&self, instance_path: &str) -> Result<GamepadState> {
        let slot = parse_xinput_slot(instance_path)?;
        let guard = self.xinput.lock().unwrap();
        let handle = guard
            .as_ref()
            .ok_or_else(|| PadSwitchError::Platform("XInput not loaded".into()))?;

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
        Ok(())
    }
}

/// Try to extract an XInput slot from a device identifier.
/// Supports both legacy "XINPUT\SLOT{n}" paths and numeric slot strings.
fn parse_xinput_slot(instance_path: &str) -> Result<u32> {
    let upper = instance_path.to_uppercase();
    if let Some(rest) = upper.strip_prefix("XINPUT\\SLOT") {
        return rest.parse::<u32>().map_err(|_| {
            PadSwitchError::Platform(format!("Invalid XInput slot in path: {}", instance_path))
        });
    }
    // Try parsing as plain slot number
    if let Ok(slot) = instance_path.parse::<u32>() {
        if slot < 4 {
            return Ok(slot);
        }
    }
    Err(PadSwitchError::Platform(format!(
        "Cannot determine XInput slot from: {}",
        instance_path
    )))
}
