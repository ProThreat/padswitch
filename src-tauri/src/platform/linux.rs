use crate::device::{DeviceType, DriverStatus, GamepadState, PhysicalDevice};
use crate::error::{PadSwitchError, Result};
use crate::platform::{DeviceEnumerator, DeviceHider, VirtualControllerManager};
use evdev::{AbsoluteAxisCode, KeyCode};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Linux platform backend using evdev for physical device enumeration
/// and uinput for virtual controller creation (in the input loop).
pub struct LinuxPlatform;

impl LinuxPlatform {
    pub fn new() -> Self {
        Self
    }
}

/// Check if an evdev device looks like a gamepad by inspecting its supported keys.
fn is_gamepad(device: &evdev::Device) -> bool {
    let Some(keys) = device.supported_keys() else {
        return false;
    };
    keys.contains(KeyCode::BTN_GAMEPAD) || keys.contains(KeyCode::BTN_SOUTH)
}

/// Generate a stable device ID by hashing the physical path (or name+vid+pid as fallback).
fn stable_device_id(device: &evdev::Device) -> String {
    let mut hasher = DefaultHasher::new();

    if let Some(phys) = device.physical_path() {
        if !phys.is_empty() {
            phys.hash(&mut hasher);
            return format!("linux-{:016x}", hasher.finish());
        }
    }

    // Fallback: hash name + vendor + product
    let id = device.input_id();
    device.name().unwrap_or("unknown").hash(&mut hasher);
    id.vendor().hash(&mut hasher);
    id.product().hash(&mut hasher);
    format!("linux-{:016x}", hasher.finish())
}

impl DeviceEnumerator for LinuxPlatform {
    fn enumerate_devices(&self) -> Result<Vec<PhysicalDevice>> {
        let mut devices = Vec::new();

        for (path, device) in evdev::enumerate() {
            if !is_gamepad(&device) {
                continue;
            }

            let id = device.input_id();
            let name = device
                .name()
                .unwrap_or("Unknown Gamepad")
                .to_string();
            let instance_path = path.to_string_lossy().to_string();

            devices.push(PhysicalDevice {
                id: stable_device_id(&device),
                name,
                instance_path,
                device_type: DeviceType::XInput, // Linux doesn't distinguish XInput/DirectInput
                hidden: false,
                connected: true,
                vendor_id: id.vendor(),
                product_id: id.product(),
                xinput_slot: None, // No XInput slots on Linux
            });
        }

        Ok(devices)
    }

    fn check_drivers(&self) -> Result<DriverStatus> {
        // On Linux, Force mode needs /dev/uinput. No external drivers like HidHide/ViGEm.
        // Return true for both fields to prevent false "driver missing" warnings in the UI.
        let uinput_writable = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/uinput")
            .is_ok();

        Ok(DriverStatus {
            hidhide_installed: true, // N/A on Linux — EVIOCGRAB replaces HidHide
            vigembus_installed: uinput_writable, // uinput replaces ViGEmBus
            hidhide_version: None,
            vigembus_version: if uinput_writable {
                Some("uinput (kernel module)".into())
            } else {
                None
            },
        })
    }
}

impl DeviceHider for LinuxPlatform {
    fn hide_device(&self, _instance_path: &str) -> Result<()> {
        // Hiding is done via EVIOCGRAB in the input loop, not here
        Ok(())
    }

    fn unhide_device(&self, _instance_path: &str) -> Result<()> {
        // Grab is released when the device fd is dropped in the input loop
        Ok(())
    }

    fn whitelist_self(&self) -> Result<()> {
        // No whitelist concept on Linux — we grab devices directly
        Ok(())
    }

    fn disable_device(&self, _instance_path: &str) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported(
            "Minimal mode is not supported on Linux. Use Force mode instead.".into(),
        ))
    }

    fn enable_device(&self, _instance_path: &str) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported(
            "Minimal mode is not supported on Linux. Use Force mode instead.".into(),
        ))
    }

    fn deactivate_hiding(&self) -> Result<()> {
        // No hiding driver to deactivate on Linux
        Ok(())
    }
}

impl VirtualControllerManager for LinuxPlatform {
    fn create_virtual_controller(&self) -> Result<u32> {
        // Virtual controllers are created in the input loop thread (same pattern as Windows/ViGEm)
        Err(PadSwitchError::PlatformNotSupported(
            "Virtual controllers are managed by the input loop on Linux".into(),
        ))
    }

    fn destroy_virtual_controller(&self, _index: u32) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported(
            "Virtual controllers are managed by the input loop on Linux".into(),
        ))
    }

    fn read_gamepad_state(&self, instance_path: &str) -> Result<GamepadState> {
        let device = evdev::Device::open(instance_path).map_err(|e| {
            PadSwitchError::Platform(format!("Failed to open {}: {}", instance_path, e))
        })?;

        let mut state = GamepadState::default();

        // Read absolute axis values
        if let Some(abs_state) = device.get_abs_state() {
            for info in &abs_state {
                // Map standard gamepad axes to GamepadState fields.
                // evdev absolute axis values vary by device; normalize to XInput ranges.
                match AbsoluteAxisCode(info.code) {
                    AbsoluteAxisCode::ABS_X => state.thumb_lx = normalize_axis(info.value, info.minimum, info.maximum),
                    AbsoluteAxisCode::ABS_Y => state.thumb_ly = normalize_axis_inverted(info.value, info.minimum, info.maximum),
                    AbsoluteAxisCode::ABS_RX => state.thumb_rx = normalize_axis(info.value, info.minimum, info.maximum),
                    AbsoluteAxisCode::ABS_RY => state.thumb_ry = normalize_axis_inverted(info.value, info.minimum, info.maximum),
                    AbsoluteAxisCode::ABS_Z => state.left_trigger = normalize_trigger(info.value, info.minimum, info.maximum),
                    AbsoluteAxisCode::ABS_RZ => state.right_trigger = normalize_trigger(info.value, info.minimum, info.maximum),
                    _ => {}
                }
            }
        }

        // Read button state
        if let Some(keys) = device.get_key_state() {
            state.buttons = map_evdev_buttons_to_xinput(&keys);
        }

        Ok(state)
    }

    fn write_virtual_state(&self, _index: u32, _state: &GamepadState) -> Result<()> {
        Err(PadSwitchError::PlatformNotSupported(
            "Virtual controllers are managed by the input loop on Linux".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Axis / button mapping helpers
// ---------------------------------------------------------------------------

/// Information about an absolute axis from evdev's cached state.
struct AbsAxisInfo {
    code: u16,
    value: i32,
    minimum: i32,
    maximum: i32,
}

/// Extension trait to read cached absolute axis state from an evdev device.
trait AbsStateExt {
    fn get_abs_state(&self) -> Option<Vec<AbsAxisInfo>>;
    fn get_key_state(&self) -> Option<evdev::AttributeSet<KeyCode>>;
}

impl AbsStateExt for evdev::Device {
    fn get_abs_state(&self) -> Option<Vec<AbsAxisInfo>> {
        let supported = self.supported_absolute_axes()?;
        let mut result = Vec::new();
        for axis in supported.iter() {
            if let Some(info) = self.get_abs_state_by_code(&axis) {
                result.push(AbsAxisInfo {
                    code: axis.0,
                    value: info.value,
                    minimum: info.minimum,
                    maximum: info.maximum,
                });
            }
        }
        Some(result)
    }

    fn get_key_state(&self) -> Option<evdev::AttributeSet<KeyCode>> {
        self.cached_state().key_vals()
    }
}

/// Helper to read a single axis's AbsInfo from the device.
trait AbsInfoExt {
    fn get_abs_state_by_code(&self, code: &AbsoluteAxisCode) -> Option<AbsAxisInfoSimple>;
}

struct AbsAxisInfoSimple {
    value: i32,
    minimum: i32,
    maximum: i32,
}

impl AbsInfoExt for evdev::Device {
    fn get_abs_state_by_code(&self, code: &AbsoluteAxisCode) -> Option<AbsAxisInfoSimple> {
        let state = self.cached_state();
        let info = self.get_absinfo(code)?;
        let value = state
            .abs_vals()
            .and_then(|vals| vals.get(*code))
            .map(|ai| ai.value)
            .unwrap_or(info.value());
        Some(AbsAxisInfoSimple {
            value,
            minimum: info.minimum(),
            maximum: info.maximum(),
        })
    }
}

/// Normalize an evdev axis value (min..max) to XInput i16 range (-32768..32767).
fn normalize_axis(value: i32, min: i32, max: i32) -> i16 {
    if max == min {
        return 0;
    }
    let normalized = (value - min) as f64 / (max - min) as f64; // 0.0 .. 1.0
    let xinput = normalized * 65535.0 - 32768.0; // -32768 .. 32767
    xinput.round().clamp(-32768.0, 32767.0) as i16
}

/// Same as normalize_axis but inverted (Y axes are often inverted between evdev and XInput).
fn normalize_axis_inverted(value: i32, min: i32, max: i32) -> i16 {
    let n = normalize_axis(value, min, max);
    if n == i16::MIN {
        i16::MAX
    } else {
        -n
    }
}

/// Normalize an evdev trigger value (min..max) to XInput u8 range (0..255).
fn normalize_trigger(value: i32, min: i32, max: i32) -> u8 {
    if max == min {
        return 0;
    }
    let normalized = (value - min) as f64 / (max - min) as f64; // 0.0 .. 1.0
    (normalized * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Map evdev key state to XInput button bitmask.
fn map_evdev_buttons_to_xinput(keys: &evdev::AttributeSet<KeyCode>) -> u16 {
    let mut buttons: u16 = 0;

    // XInput button constants (matching Windows XINPUT_GAMEPAD_*)
    const DPAD_UP: u16 = 0x0001;
    const DPAD_DOWN: u16 = 0x0002;
    const DPAD_LEFT: u16 = 0x0004;
    const DPAD_RIGHT: u16 = 0x0008;
    const START: u16 = 0x0010;
    const BACK: u16 = 0x0020;
    const LEFT_THUMB: u16 = 0x0040;
    const RIGHT_THUMB: u16 = 0x0080;
    const LEFT_SHOULDER: u16 = 0x0100;
    const RIGHT_SHOULDER: u16 = 0x0200;
    const A: u16 = 0x1000;
    const B: u16 = 0x2000;
    const X: u16 = 0x4000;
    const Y: u16 = 0x8000;

    if keys.contains(KeyCode::BTN_SOUTH) { buttons |= A; }
    if keys.contains(KeyCode::BTN_EAST) { buttons |= B; }
    if keys.contains(KeyCode::BTN_WEST) { buttons |= X; }
    if keys.contains(KeyCode::BTN_NORTH) { buttons |= Y; }
    if keys.contains(KeyCode::BTN_TL) { buttons |= LEFT_SHOULDER; }
    if keys.contains(KeyCode::BTN_TR) { buttons |= RIGHT_SHOULDER; }
    if keys.contains(KeyCode::BTN_SELECT) { buttons |= BACK; }
    if keys.contains(KeyCode::BTN_START) { buttons |= START; }
    if keys.contains(KeyCode::BTN_THUMBL) { buttons |= LEFT_THUMB; }
    if keys.contains(KeyCode::BTN_THUMBR) { buttons |= RIGHT_THUMB; }
    if keys.contains(KeyCode::BTN_DPAD_UP) { buttons |= DPAD_UP; }
    if keys.contains(KeyCode::BTN_DPAD_DOWN) { buttons |= DPAD_DOWN; }
    if keys.contains(KeyCode::BTN_DPAD_LEFT) { buttons |= DPAD_LEFT; }
    if keys.contains(KeyCode::BTN_DPAD_RIGHT) { buttons |= DPAD_RIGHT; }

    buttons
}
