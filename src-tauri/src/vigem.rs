/// ViGEmBus wrapper (Windows-only).
///
/// ViGEmBus is a virtual gamepad bus driver by Nefarius (Benjamin Höglinger-Stelzer)
/// that allows creating virtual Xbox 360 / DualShock 4 controllers.
///
/// This module provides stateless helpers only. The actual Client + Xbox360Wired
/// targets are created and owned inside the input loop thread to avoid
/// self-referencing lifetime issues (Xbox360Wired borrows &Client).
///
/// Reference: https://github.com/nefarius/ViGEmBus
/// Rust crate: https://github.com/CasualX/vigem-client

#[cfg(target_os = "windows")]
pub mod imp {
    use crate::device::GamepadState;

    /// Check if ViGEmBus is installed by attempting to connect.
    pub fn is_installed() -> bool {
        vigem_client::Client::connect().is_ok()
    }

    /// Map a PadSwitch GamepadState to a vigem_client XGamepad.
    pub fn to_xgamepad(state: &GamepadState) -> vigem_client::XGamepad {
        vigem_client::XGamepad {
            buttons: vigem_client::XButtons(state.buttons),
            left_trigger: state.left_trigger,
            right_trigger: state.right_trigger,
            thumb_lx: state.thumb_lx,
            thumb_ly: state.thumb_ly,
            thumb_rx: state.thumb_rx,
            thumb_ry: state.thumb_ry,
        }
    }
}
