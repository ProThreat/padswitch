/// SetupDi device enable/disable + real device enumeration (Windows-only).
///
/// Uses the Windows SetupAPI to:
/// 1. Enumerate real game controller devices with their actual instance paths
/// 2. Disable and re-enable physical devices (for minimal mode reordering)
///
/// Note: SetupDi enable/disable typically requires admin elevation.

#[cfg(target_os = "windows")]
pub mod imp {
    use crate::device::PhysicalDevice;
    use crate::error::{PadSwitchError, Result};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use windows::core::PCWSTR;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiCallClassInstaller, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
        SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW,
        SetupDiSetClassInstallParamsW, DIF_PROPERTYCHANGE, DIGCF_ALLCLASSES, DIGCF_PRESENT,
        DICS_DISABLE, DICS_ENABLE, DICS_FLAG_GLOBAL, DI_FUNCTION, SETUP_DI_REGISTRY_PROPERTY,
        SP_CLASSINSTALL_HEADER, SP_DEVINFO_DATA, SP_PROPCHANGE_PARAMS, SPDRP_CLASS,
        SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME, SPDRP_HARDWAREID, SPDRP_SERVICE,
    };

    /// Info about a game controller discovered via SetupAPI.
    pub struct GameControllerInfo {
        pub instance_path: String,
        pub name: String,
        pub vendor_id: u16,
        pub product_id: u16,
        /// Whether this device uses an XInput-compatible driver (XUSB/XINPUT/XBOXGIP).
        /// Only XInput devices occupy XInput slots 0-3.
        pub is_xinput: bool,
    }

    /// Generate a stable device ID from the instance path (deterministic across sessions).
    pub fn stable_device_id(instance_path: &str) -> String {
        let mut hasher = DefaultHasher::new();
        instance_path.to_uppercase().hash(&mut hasher);
        format!("dev-{:016x}", hasher.finish())
    }

    /// Enumerate real game controller devices via SetupAPI.
    /// Finds XInput-compatible controllers by checking driver service names,
    /// device class names, and device descriptions.
    pub fn enumerate_game_controllers() -> Result<Vec<GameControllerInfo>> {
        unsafe {
            let dev_info = SetupDiGetClassDevsW(
                None,
                PCWSTR::null(),
                None,
                DIGCF_ALLCLASSES | DIGCF_PRESENT,
            )
            .map_err(|e| {
                PadSwitchError::Platform(format!("SetupDiGetClassDevsW failed: {}", e))
            })?;

            let mut controllers = Vec::new();
            let mut index: u32 = 0;

            loop {
                let mut dev_data = SP_DEVINFO_DATA {
                    cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                    ..Default::default()
                };

                if SetupDiEnumDeviceInfo(dev_info, index, &mut dev_data).is_err() {
                    break;
                }
                index += 1;

                // Get device properties
                let service = get_device_string_property(dev_info, &dev_data, SPDRP_SERVICE);
                let class = get_device_string_property(dev_info, &dev_data, SPDRP_CLASS);
                let description =
                    get_device_string_property(dev_info, &dev_data, SPDRP_DEVICEDESC);

                // Filter: is this a game controller?
                if !is_game_controller(&service, &description, &class) {
                    continue;
                }

                // Get real instance ID
                let mut id_buf = vec![0u16; 512];
                let mut required_size: u32 = 0;
                if SetupDiGetDeviceInstanceIdW(
                    dev_info,
                    &dev_data,
                    Some(&mut id_buf),
                    Some(&mut required_size),
                )
                .is_err()
                {
                    continue;
                }
                let instance_path = String::from_utf16_lossy(
                    &id_buf[..required_size.saturating_sub(1) as usize],
                );

                // Get friendly name (prefer over description)
                let friendly =
                    get_device_string_property(dev_info, &dev_data, SPDRP_FRIENDLYNAME);
                let name = if friendly.is_empty() {
                    description
                } else {
                    friendly
                };

                // Get VID/PID from hardware IDs
                let hw_ids = get_device_multi_string_property(dev_info, &dev_data, SPDRP_HARDWAREID);
                let (vid, pid) = extract_vid_pid(&hw_ids);

                // Check if this uses an XInput-compatible driver
                let is_xinput = is_xinput_driver(&service, &class);

                controllers.push(GameControllerInfo {
                    instance_path,
                    name,
                    vendor_id: vid,
                    product_id: pid,
                    is_xinput,
                });
            }

            let _ = SetupDiDestroyDeviceInfoList(dev_info);
            Ok(controllers)
        }
    }

    /// Disable a device by its real instance path.
    pub fn disable_device(instance_path: &str) -> Result<()> {
        change_device_state(instance_path, DICS_DISABLE)
    }

    /// Enable a device by its real instance path.
    pub fn enable_device(instance_path: &str) -> Result<()> {
        change_device_state(instance_path, DICS_ENABLE)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Check if a device uses an XInput-compatible driver (occupies an XInput slot 0-3).
    /// Only these devices should be assigned XInput slot numbers.
    fn is_xinput_driver(service: &str, class: &str) -> bool {
        let service_up = service.to_uppercase();
        let class_low = class.to_lowercase();

        service_up.contains("XUSB")
            || service_up.contains("XINPUT")
            || service_up == "XBOXGIP"
            || class_low.contains("xna")
            || class_low.contains("xbox")
    }

    /// Check if a device is a game controller based on its service, description, and class.
    fn is_game_controller(service: &str, description: &str, class: &str) -> bool {
        let service_up = service.to_uppercase();
        let desc_low = description.to_lowercase();
        let class_low = class.to_lowercase();

        // Xbox/XInput driver services
        if service_up.contains("XUSB")
            || service_up.contains("XINPUT")
            || service_up == "XBOXGIP"
        {
            return true;
        }

        // Xbox device class names
        if class_low.contains("xna") || class_low.contains("xbox") {
            return true;
        }

        // Game controller keywords in description
        if desc_low.contains("game controller")
            || desc_low.contains("gamepad")
            || desc_low.contains("joystick")
        {
            return true;
        }

        // "controller" in description for HID-class devices (not USB hubs etc.)
        if desc_low.contains("controller")
            && !desc_low.contains("hub")
            && !desc_low.contains("host")
            && !desc_low.contains("root")
            && (class_low.contains("hid") || class_low.is_empty())
        {
            return true;
        }

        false
    }

    /// Get a REG_SZ string property from a device via SetupDiGetDeviceRegistryPropertyW.
    unsafe fn get_device_string_property(
        dev_info: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
        dev_data: &SP_DEVINFO_DATA,
        property: SETUP_DI_REGISTRY_PROPERTY,
    ) -> String {
        let mut size: u32 = 0;

        // First call: get required size
        let _ = SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_data,
            property,
            None,
            None,
            Some(&mut size),
        );

        if size == 0 {
            return String::new();
        }

        // Second call: get data
        let mut buffer = vec![0u8; size as usize];
        if SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_data,
            property,
            None,
            Some(&mut buffer),
            None,
        )
        .is_err()
        {
            return String::new();
        }

        // Convert UTF-16LE to String
        let wide: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        String::from_utf16_lossy(&wide[..end])
    }

    /// Get a REG_MULTI_SZ multi-string property from a device.
    unsafe fn get_device_multi_string_property(
        dev_info: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
        dev_data: &SP_DEVINFO_DATA,
        property: SETUP_DI_REGISTRY_PROPERTY,
    ) -> Vec<String> {
        let mut size: u32 = 0;

        let _ = SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_data,
            property,
            None,
            None,
            Some(&mut size),
        );

        if size == 0 {
            return vec![];
        }

        let mut buffer = vec![0u8; size as usize];
        if SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_data,
            property,
            None,
            Some(&mut buffer),
            None,
        )
        .is_err()
        {
            return vec![];
        }

        // Decode double-null-terminated UTF-16LE
        let wide: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();

        let mut strings = Vec::new();
        let mut current = Vec::new();
        for &ch in &wide {
            if ch == 0 {
                if current.is_empty() {
                    break;
                }
                strings.push(String::from_utf16_lossy(&current));
                current.clear();
            } else {
                current.push(ch);
            }
        }
        strings
    }

    /// Extract VID and PID from hardware ID strings (e.g., "USB\VID_045E&PID_028E").
    fn extract_vid_pid(hw_ids: &[String]) -> (u16, u16) {
        for hwid in hw_ids {
            let upper = hwid.to_uppercase();
            let vid = extract_hex_after(&upper, "VID_");
            let pid = extract_hex_after(&upper, "PID_");
            if vid != 0 || pid != 0 {
                return (vid, pid);
            }
        }
        (0, 0)
    }

    fn extract_hex_after(s: &str, prefix: &str) -> u16 {
        if let Some(pos) = s.find(prefix) {
            let start = pos + prefix.len();
            let hex_str: String = s[start..]
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            u16::from_str_radix(&hex_str, 16).unwrap_or(0)
        } else {
            0
        }
    }

    fn change_device_state(instance_path: &str, state_change: u32) -> Result<()> {
        unsafe {
            let dev_info = SetupDiGetClassDevsW(
                None,
                PCWSTR::null(),
                None,
                DIGCF_ALLCLASSES | DIGCF_PRESENT,
            )
            .map_err(|e| {
                PadSwitchError::Platform(format!("SetupDiGetClassDevsW failed: {}", e))
            })?;

            let result = find_and_change_device(dev_info, instance_path, state_change);
            let _ = SetupDiDestroyDeviceInfoList(dev_info);
            result
        }
    }

    unsafe fn find_and_change_device(
        dev_info: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
        instance_path: &str,
        state_change: u32,
    ) -> Result<()> {
        let target_upper = instance_path.to_uppercase();
        let mut index: u32 = 0;

        loop {
            let mut dev_info_data = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };

            if SetupDiEnumDeviceInfo(dev_info, index, &mut dev_info_data).is_err() {
                break;
            }
            index += 1;

            // Get device instance ID
            let mut id_buf = vec![0u16; 512];
            let mut required_size: u32 = 0;
            if SetupDiGetDeviceInstanceIdW(
                dev_info,
                &dev_info_data,
                Some(&mut id_buf),
                Some(&mut required_size),
            )
            .is_err()
            {
                continue;
            }

            let device_id = String::from_utf16_lossy(
                &id_buf[..required_size.saturating_sub(1) as usize],
            );

            if device_id.to_uppercase() != target_upper {
                continue;
            }

            // Found the device — apply state change
            let params = SP_PROPCHANGE_PARAMS {
                ClassInstallHeader: SP_CLASSINSTALL_HEADER {
                    cbSize: std::mem::size_of::<SP_CLASSINSTALL_HEADER>() as u32,
                    InstallFunction: DI_FUNCTION(DIF_PROPERTYCHANGE.0),
                },
                StateChange: state_change,
                Scope: DICS_FLAG_GLOBAL,
                HwProfile: 0,
            };

            SetupDiSetClassInstallParamsW(
                dev_info,
                Some(&dev_info_data),
                Some(&params.ClassInstallHeader),
                std::mem::size_of::<SP_PROPCHANGE_PARAMS>() as u32,
            )
            .map_err(|e| {
                if e.code().0 as u32 == 0x80070005 {
                    PadSwitchError::Platform(
                        "Access denied. Run PadSwitch as Administrator to change device state."
                            .into(),
                    )
                } else {
                    PadSwitchError::Platform(format!(
                        "SetupDiSetClassInstallParamsW failed: {}",
                        e
                    ))
                }
            })?;

            SetupDiCallClassInstaller(DIF_PROPERTYCHANGE, dev_info, Some(&dev_info_data)).map_err(
                |e| {
                    if e.code().0 as u32 == 0x80070005 {
                        PadSwitchError::Platform(
                            "Access denied. Run PadSwitch as Administrator to change device state."
                                .into(),
                        )
                    } else {
                        PadSwitchError::Platform(format!(
                            "SetupDiCallClassInstaller failed: {}",
                            e
                        ))
                    }
                },
            )?;

            return Ok(());
        }

        Err(PadSwitchError::DeviceNotFound(format!(
            "Device not found in SetupDi: {}",
            instance_path
        )))
    }
}
