/// SetupDi device enable/disable (Windows-only, minimal mode).
///
/// Uses the Windows SetupAPI to disable and re-enable physical game controllers.
/// This is the least-invasive approach: no third-party drivers needed.
/// Disabling then re-enabling in the desired order causes Windows/XInput
/// to reassign slot numbers.
///
/// Note: SetupDi enable/disable typically requires admin elevation.

#[cfg(target_os = "windows")]
pub mod imp {
    use crate::device::PhysicalDevice;
    use crate::error::{PadSwitchError, Result};
    use windows::core::PCWSTR;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiCallClassInstaller, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
        SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiSetClassInstallParamsW,
        DIF_PROPERTYCHANGE, DIGCF_ALLCLASSES, DIGCF_PRESENT, DICS_DISABLE, DICS_ENABLE,
        DICS_FLAG_GLOBAL, DI_FUNCTION, SP_CLASSINSTALL_HEADER, SP_DEVINFO_DATA,
        SP_PROPCHANGE_PARAMS,
    };

    /// Disable a device by its instance path (e.g., "USB\\VID_045E&PID_028E\\...").
    pub fn disable_device(instance_path: &str) -> Result<()> {
        change_device_state(instance_path, DICS_DISABLE)
    }

    /// Enable a device by its instance path.
    pub fn enable_device(instance_path: &str) -> Result<()> {
        change_device_state(instance_path, DICS_ENABLE)
    }

    /// Enumerate connected XInput devices by probing slots 0-3 via rusty-xinput.
    pub fn enumerate_xinput_devices() -> Result<Vec<PhysicalDevice>> {
        let handle = rusty_xinput::XInputHandle::load_default()
            .map_err(|e| PadSwitchError::Platform(format!("Failed to load XInput: {:?}", e)))?;

        let mut devices = Vec::new();
        for slot in 0..4u32 {
            if handle.get_state(slot).is_ok() {
                devices.push(PhysicalDevice::from_xinput_slot(slot));
            }
        }
        Ok(devices)
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

            let result = find_and_change_device(dev_info.0 as isize, instance_path, state_change);

            let _ = SetupDiDestroyDeviceInfoList(dev_info);

            result
        }
    }

    unsafe fn find_and_change_device(
        dev_info_raw: isize,
        instance_path: &str,
        state_change: u32,
    ) -> Result<()> {
        // Reconstruct HDEVINFO from raw isize
        let dev_info =
            windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO(dev_info_raw as *mut _);

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
            let mut params = SP_PROPCHANGE_PARAMS {
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
                    // ERROR_ACCESS_DENIED
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
