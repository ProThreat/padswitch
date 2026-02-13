use crate::config::RoutingMode;
use crate::device::SlotAssignment;
use crate::error::Result;
use crate::platform::PlatformServices;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Manages the input forwarding loop.
///
/// Runs on a dedicated `std::thread` (NOT tokio) for consistent sub-ms timing.
/// Supports two modes:
/// - **Minimal**: Disable/re-enable physical devices in desired order via SetupDi.
/// - **Force**: HidHide + ViGEm virtual controllers + input forwarding at ~1000Hz.
pub struct InputLoop {
    running: Arc<AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl InputLoop {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    /// Start the forwarding loop with the given assignments and routing mode.
    pub fn start(
        &mut self,
        manager: Arc<dyn PlatformServices>,
        assignments: Vec<SlotAssignment>,
        mode: RoutingMode,
    ) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        let handle = std::thread::Builder::new()
            .name("padswitch-input-loop".into())
            .spawn(move || match mode {
                RoutingMode::Minimal => run_minimal(running, assignments),
                RoutingMode::Force => run_force_forwarding(running, manager, assignments),
            })
            .map_err(|e| {
                self.running.store(false, Ordering::SeqCst);
                crate::error::PadSwitchError::Forwarding(format!(
                    "Failed to spawn input loop: {}",
                    e
                ))
            })?;

        self.thread_handle = Some(handle);
        Ok(())
    }

    /// Stop the forwarding loop.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for InputLoop {
    fn drop(&mut self) {
        self.stop();
    }
}

// ---------------------------------------------------------------------------
// Minimal mode: disable/re-enable devices via SetupDi
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn run_minimal(running: Arc<AtomicBool>, assignments: Vec<SlotAssignment>) {
    use crate::setupdi::imp;

    log::info!(
        "Minimal mode: reordering {} devices",
        assignments.len()
    );

    // Sort by target slot
    let mut sorted = assignments.clone();
    sorted.sort_by_key(|a| a.slot);

    // Collect instance paths (we use the convention XINPUT\SLOT{n})
    let paths: Vec<String> = sorted
        .iter()
        .map(|a| format!("XINPUT\\SLOT{}", a.slot))
        .collect();

    // Step 1: Disable all assigned devices
    for path in &paths {
        if let Err(e) = imp::disable_device(path) {
            log::error!("Failed to disable {}: {}", path, e);
        }
    }

    // Step 2: Wait for OS to process
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Step 3: Re-enable each device in the desired order with delays
    for path in &paths {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        if let Err(e) = imp::enable_device(path) {
            log::error!("Failed to enable {}: {}", path, e);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    log::info!("Minimal mode: reorder complete, holding state");

    // Hold state — thread stays alive so stop() can clean up
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // Cleanup: re-enable all devices in case any were left disabled
    log::info!("Minimal mode: cleanup — re-enabling all devices");
    for path in &paths {
        if let Err(e) = imp::enable_device(path) {
            log::warn!("Cleanup enable failed for {}: {}", path, e);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn run_minimal(running: Arc<AtomicBool>, _assignments: Vec<SlotAssignment>) {
    log::info!("Minimal mode: stub (non-Windows)");
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

// ---------------------------------------------------------------------------
// Force mode: HidHide + ViGEm + input forwarding loop
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn run_force_forwarding(
    running: Arc<AtomicBool>,
    manager: Arc<dyn PlatformServices>,
    assignments: Vec<SlotAssignment>,
) {
    use crate::hidhide::imp::HidHide;
    use crate::vigem::imp::to_xgamepad;

    log::info!(
        "Force mode: starting with {} assignments",
        assignments.len()
    );

    // Sort assignments by target slot
    let mut sorted = assignments.clone();
    sorted.sort_by_key(|a| a.slot);

    // Step 1: Whitelist ourselves so we can still read hidden devices
    if let Err(e) = manager.whitelist_self() {
        log::error!("Failed to whitelist self: {}", e);
        running.store(false, Ordering::SeqCst);
        return;
    }

    // Step 2: Hide all assigned physical devices
    let instance_paths: Vec<String> = sorted
        .iter()
        .map(|a| format!("XINPUT\\SLOT{}", a.slot))
        .collect();

    for path in &instance_paths {
        if let Err(e) = manager.hide_device(path) {
            log::error!("Failed to hide {}: {}", path, e);
        }
    }

    // Step 3: Activate HidHide
    match HidHide::open() {
        Ok(hh) => {
            if let Err(e) = hh.set_active(true) {
                log::error!("Failed to activate HidHide: {}", e);
            }
        }
        Err(e) => {
            log::error!("Failed to open HidHide for activation: {}", e);
            running.store(false, Ordering::SeqCst);
            return;
        }
    }

    // Step 4: Connect to ViGEmBus — client lives on this thread's stack
    let client = match vigem_client::Client::connect() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to connect to ViGEmBus: {:?}", e);
            cleanup_force(&manager, &instance_paths);
            running.store(false, Ordering::SeqCst);
            return;
        }
    };

    // Step 5: Create virtual Xbox 360 targets in slot order
    let mut targets: Vec<vigem_client::Xbox360Wired<'_>> = Vec::new();
    for _ in &sorted {
        let mut target = vigem_client::Xbox360Wired::new(&client, vigem_client::TargetId::XBOX360_WIRED);
        match target.plugin_wait() {
            Ok(()) => targets.push(target),
            Err(e) => {
                log::error!("Failed to plug in virtual controller: {:?}", e);
                cleanup_force(&manager, &instance_paths);
                running.store(false, Ordering::SeqCst);
                return;
            }
        }
    }

    // Step 6: Load XInput handle for reading physical state
    let xinput = match rusty_xinput::XInputHandle::load_default() {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to load XInput: {:?}", e);
            cleanup_force(&manager, &instance_paths);
            running.store(false, Ordering::SeqCst);
            return;
        }
    };

    log::info!("Force mode: forwarding loop active");

    // Step 7: Poll loop at ~1000Hz
    while running.load(Ordering::SeqCst) {
        for (i, assignment) in sorted.iter().enumerate() {
            if let Ok(state) = xinput.get_state(assignment.slot as u32) {
                let gamepad = crate::device::GamepadState {
                    buttons: state.raw.Gamepad.wButtons,
                    left_trigger: state.raw.Gamepad.bLeftTrigger,
                    right_trigger: state.raw.Gamepad.bRightTrigger,
                    thumb_lx: state.raw.Gamepad.sThumbLX,
                    thumb_ly: state.raw.Gamepad.sThumbLY,
                    thumb_rx: state.raw.Gamepad.sThumbRX,
                    thumb_ry: state.raw.Gamepad.sThumbRY,
                };
                let xgamepad = to_xgamepad(&gamepad);
                let _ = targets[i].update(&xgamepad);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    log::info!("Force mode: stopping — cleaning up");

    // Step 8: Drop targets (unplugs virtual controllers), then unhide devices
    drop(targets);
    cleanup_force(&manager, &instance_paths);
}

#[cfg(target_os = "windows")]
fn cleanup_force(manager: &Arc<dyn PlatformServices>, instance_paths: &[String]) {
    use crate::hidhide::imp::HidHide;

    // Deactivate HidHide
    if let Ok(hh) = HidHide::open() {
        let _ = hh.set_active(false);
    }

    // Unhide all devices
    for path in instance_paths {
        if let Err(e) = manager.unhide_device(path) {
            log::warn!("Cleanup unhide failed for {}: {}", path, e);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn run_force_forwarding(
    running: Arc<AtomicBool>,
    _manager: Arc<dyn PlatformServices>,
    _assignments: Vec<SlotAssignment>,
) {
    log::info!("Force mode: stub (non-Windows)");
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
