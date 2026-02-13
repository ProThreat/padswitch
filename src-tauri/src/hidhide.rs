/// HidHide IOCTL wrapper (Windows-only).
///
/// HidHide is a filter driver by Nefarius (Benjamin Höglinger-Stelzer)
/// that can hide HID devices from applications while allowing whitelisted
/// apps to still access them.
///
/// Key operations:
/// - Get/set blacklist (device instance paths to hide)
/// - Get/set whitelist (application paths allowed to see hidden devices)
/// - Enable/disable hiding globally
///
/// All string lists use double-null-terminated UTF-16LE encoding.
///
/// Reference: https://github.com/nefarius/HidHide

#[cfg(target_os = "windows")]
pub mod imp {
    use crate::error::{PadSwitchError, Result};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::IO::DeviceIoControl;

    const HIDHIDE_DEVICE_PATH: &str = r"\\.\HidHide";

    // IOCTL codes for HidHide
    const IOCTL_GET_WHITELIST: u32 = 0x80016000;
    const IOCTL_SET_WHITELIST: u32 = 0x80016004;
    const IOCTL_GET_BLACKLIST: u32 = 0x80016008;
    const IOCTL_SET_BLACKLIST: u32 = 0x8001600C;
    #[allow(dead_code)]
    const IOCTL_GET_ACTIVE: u32 = 0x80016010;
    const IOCTL_SET_ACTIVE: u32 = 0x80016014;

    pub struct HidHide {
        handle: HANDLE,
    }

    impl Drop for HidHide {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }

    impl HidHide {
        /// Open a handle to the HidHide device. HidHide only allows one handle
        /// at a time, so callers should open/close per operation.
        pub fn open() -> Result<Self> {
            let path: Vec<u16> = HIDHIDE_DEVICE_PATH
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let handle = unsafe {
                CreateFileW(
                    PCWSTR(path.as_ptr()),
                    0, // No read/write needed for IOCTLs
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                )
                .map_err(|e| PadSwitchError::HidHide(format!("Failed to open HidHide: {}", e)))?
            };
            Ok(Self { handle })
        }

        /// Check if HidHide is installed by attempting to open its device path.
        pub fn is_installed() -> bool {
            Self::open().is_ok()
        }

        /// Add a device instance path to the blacklist (devices to hide).
        pub fn add_to_blacklist(&self, instance_path: &str) -> Result<()> {
            let mut list = self.ioctl_get_list(IOCTL_GET_BLACKLIST)?;
            let normalized = instance_path.to_uppercase();
            if !list.iter().any(|s| s.to_uppercase() == normalized) {
                list.push(instance_path.to_string());
                self.ioctl_set_list(IOCTL_SET_BLACKLIST, &list)?;
            }
            Ok(())
        }

        /// Remove a device instance path from the blacklist.
        pub fn remove_from_blacklist(&self, instance_path: &str) -> Result<()> {
            let mut list = self.ioctl_get_list(IOCTL_GET_BLACKLIST)?;
            let normalized = instance_path.to_uppercase();
            let before = list.len();
            list.retain(|s| s.to_uppercase() != normalized);
            if list.len() != before {
                self.ioctl_set_list(IOCTL_SET_BLACKLIST, &list)?;
            }
            Ok(())
        }

        /// Add an application path to the whitelist (apps allowed to see hidden devices).
        pub fn add_to_whitelist(&self, app_path: &str) -> Result<()> {
            let mut list = self.ioctl_get_list(IOCTL_GET_WHITELIST)?;
            let normalized = app_path.to_uppercase();
            if !list.iter().any(|s| s.to_uppercase() == normalized) {
                list.push(app_path.to_string());
                self.ioctl_set_list(IOCTL_SET_WHITELIST, &list)?;
            }
            Ok(())
        }

        /// Enable or disable HidHide globally.
        pub fn set_active(&self, active: bool) -> Result<()> {
            let value: u8 = if active { 1 } else { 0 };
            let mut bytes_returned: u32 = 0;
            unsafe {
                DeviceIoControl(
                    self.handle,
                    IOCTL_SET_ACTIVE,
                    Some(&value as *const u8 as *const _),
                    std::mem::size_of::<u8>() as u32,
                    None,
                    0,
                    Some(&mut bytes_returned),
                    None,
                )
                .map_err(|e| PadSwitchError::HidHide(format!("set_active failed: {}", e)))?;
            }
            Ok(())
        }

        /// Get a multi-string list via IOCTL (two-call pattern: get size, then get data).
        fn ioctl_get_list(&self, ioctl_code: u32) -> Result<Vec<String>> {
            let mut bytes_returned: u32 = 0;

            // First call: get required buffer size
            let _ = unsafe {
                DeviceIoControl(
                    self.handle,
                    ioctl_code,
                    None,
                    0,
                    None,
                    0,
                    Some(&mut bytes_returned),
                    None,
                )
            };

            if bytes_returned == 0 {
                return Ok(vec![]);
            }

            // Second call: get actual data
            let mut buffer = vec![0u8; bytes_returned as usize];
            unsafe {
                DeviceIoControl(
                    self.handle,
                    ioctl_code,
                    None,
                    0,
                    Some(buffer.as_mut_ptr() as *mut _),
                    buffer.len() as u32,
                    Some(&mut bytes_returned),
                    None,
                )
                .map_err(|e| PadSwitchError::HidHide(format!("ioctl_get_list failed: {}", e)))?;
            }

            buffer.truncate(bytes_returned as usize);
            Ok(decode_multi_string(&buffer))
        }

        /// Set a multi-string list via IOCTL.
        fn ioctl_set_list(&self, ioctl_code: u32, strings: &[String]) -> Result<()> {
            let buffer = encode_multi_string(strings);
            let mut bytes_returned: u32 = 0;
            unsafe {
                DeviceIoControl(
                    self.handle,
                    ioctl_code,
                    Some(buffer.as_ptr() as *const _),
                    buffer.len() as u32,
                    None,
                    0,
                    Some(&mut bytes_returned),
                    None,
                )
                .map_err(|e| PadSwitchError::HidHide(format!("ioctl_set_list failed: {}", e)))?;
            }
            Ok(())
        }
    }

    /// Encode a list of strings as a double-null-terminated UTF-16LE byte buffer.
    fn encode_multi_string(strings: &[String]) -> Vec<u8> {
        let mut wide: Vec<u16> = Vec::new();
        for s in strings {
            wide.extend(s.encode_utf16());
            wide.push(0); // null-terminate each string
        }
        wide.push(0); // final null for double-null termination

        // Convert to bytes (little-endian)
        wide.iter().flat_map(|&w| w.to_le_bytes()).collect()
    }

    /// Decode a double-null-terminated UTF-16LE byte buffer into strings.
    fn decode_multi_string(bytes: &[u8]) -> Vec<String> {
        if bytes.len() < 2 {
            return vec![];
        }

        // Convert bytes to u16 values
        let wide: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        let mut strings = Vec::new();
        let mut current = Vec::new();
        for &ch in &wide {
            if ch == 0 {
                if current.is_empty() {
                    break; // double null = end of list
                }
                strings.push(String::from_utf16_lossy(&current));
                current.clear();
            } else {
                current.push(ch);
            }
        }
        strings
    }
}
