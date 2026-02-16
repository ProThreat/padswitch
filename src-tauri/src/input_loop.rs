use crate::config::RoutingMode;
use crate::error::Result;
use crate::platform::PlatformServices;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A slot assignment resolved to real device data for the input loop.
/// Created by commands.rs from SlotAssignment + device list lookup.
#[derive(Debug, Clone)]
pub struct ResolvedAssignment {
    /// Real device instance path (e.g., "USB\VID_045E&PID_028E\6&ABC")
    pub instance_path: String,
    /// XInput slot this device currently occupies (0-3), if known
    pub xinput_slot: Option<u32>,
    /// Target virtual slot (0-3)
    pub target_slot: u8,
}

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

    /// Start the forwarding loop with resolved assignments and routing mode.
    pub fn start(
        &mut self,
        manager: Arc<dyn PlatformServices>,
        assignments: Vec<ResolvedAssignment>,
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
fn run_minimal(running: Arc<AtomicBool>, assignments: Vec<ResolvedAssignment>) {
    use crate::setupdi::imp;

    log::info!(
        "Minimal mode: reordering {} devices",
        assignments.len()
    );

    // Sort by target slot so devices re-enable in the desired XInput order
    let mut sorted = assignments.clone();
    sorted.sort_by_key(|a| a.target_slot);

    // Use real instance paths for SetupDi operations
    let paths: Vec<&str> = sorted.iter().map(|a| a.instance_path.as_str()).collect();

    // Step 1: Disable all assigned devices
    for path in &paths {
        log::info!("Minimal mode: disabling {}", path);
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
        log::info!("Minimal mode: re-enabling {}", path);
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

#[cfg(target_os = "linux")]
fn run_minimal(running: Arc<AtomicBool>, _assignments: Vec<ResolvedAssignment>) {
    // Minimal mode is not supported on Linux — the preflight check in state.rs
    // should already block this, but log an error defensively.
    log::error!("Minimal mode is not supported on Linux. Use Force mode instead.");
    running.store(false, Ordering::SeqCst);
}

#[cfg(target_os = "macos")]
fn run_minimal(running: Arc<AtomicBool>, _assignments: Vec<ResolvedAssignment>) {
    log::info!("Minimal mode: stub (macOS)");
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
    assignments: Vec<ResolvedAssignment>,
) {
    use crate::hidhide::imp::HidHide;
    use crate::vigem::imp::to_xgamepad;

    log::info!(
        "Force mode: starting with {} assignments",
        assignments.len()
    );

    // Sort assignments by target slot
    let mut sorted = assignments.clone();
    sorted.sort_by_key(|a| a.target_slot);

    // Step 1: Whitelist ourselves so we can still read hidden devices
    if let Err(e) = manager.whitelist_self() {
        log::error!("Failed to whitelist self: {}", e);
        running.store(false, Ordering::SeqCst);
        return;
    }

    // Step 2: Hide all assigned physical devices using real instance paths
    let instance_paths: Vec<String> = sorted.iter().map(|a| a.instance_path.clone()).collect();

    for path in &instance_paths {
        log::info!("Force mode: hiding {}", path);
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

    // Step 7: Poll loop at ~1000Hz — read from real XInput slots, write to virtual targets
    while running.load(Ordering::SeqCst) {
        for (i, ra) in sorted.iter().enumerate() {
            let Some(slot) = ra.xinput_slot else {
                continue; // Skip devices without a known XInput slot
            };
            if let Ok(state) = xinput.get_state(slot) {
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

#[cfg(target_os = "linux")]
fn run_force_forwarding(
    running: Arc<AtomicBool>,
    _manager: Arc<dyn PlatformServices>,
    assignments: Vec<ResolvedAssignment>,
) {
    use evdev::uinput::VirtualDeviceBuilder;
    use evdev::{AbsoluteAxisCode, AbsInfo, UinputAbsSetup, InputEvent, EventType};

    log::info!(
        "Force mode (Linux): starting with {} assignments",
        assignments.len()
    );

    // Sort assignments by target slot so virtual devices are created in P1, P2, ... order
    let mut sorted = assignments.clone();
    sorted.sort_by_key(|a| a.target_slot);

    // Step 1: Open and grab all physical devices
    let mut physical_devices: Vec<evdev::Device> = Vec::new();
    for ra in &sorted {
        let mut device = match evdev::Device::open(&ra.instance_path) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to open {}: {}", ra.instance_path, e);
                // Release any already-grabbed devices
                drop(physical_devices);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        // EVIOCGRAB — exclusive access, other apps (games) won't see this device
        if let Err(e) = device.grab() {
            log::error!("Failed to grab {}: {}", ra.instance_path, e);
            drop(physical_devices);
            running.store(false, Ordering::SeqCst);
            return;
        }
        log::info!("Grabbed: {} ({})", ra.instance_path, device.name().unwrap_or("?"));

        physical_devices.push(device);
    }

    // Step 2: Create virtual uinput devices, one per physical device, in slot order
    let mut virtual_devices: Vec<evdev::uinput::VirtualDevice> = Vec::new();
    for (i, phys) in physical_devices.iter().enumerate() {
        let virt_name = format!("PadSwitch Virtual Controller {}", i + 1);
        let mut builder = VirtualDeviceBuilder::new()
            .map_err(|e| {
                log::error!("Failed to create VirtualDeviceBuilder: {}", e);
            });

        let mut builder = match builder {
            Ok(b) => b,
            Err(()) => {
                drop(virtual_devices);
                drop(physical_devices);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        builder = builder.name(&virt_name);

        // Copy supported keys from physical device
        if let Some(keys) = phys.supported_keys() {
            builder = builder.with_keys(&keys).unwrap_or(builder);
        }

        // Copy absolute axes with their ranges from physical device
        if let Some(abs_axes) = phys.supported_absolute_axes() {
            for axis in abs_axes.iter() {
                if let Some(info) = phys.get_absinfo(&axis) {
                    let setup = UinputAbsSetup::new(
                        axis,
                        AbsInfo::new(
                            info.value(),
                            info.minimum(),
                            info.maximum(),
                            info.fuzz(),
                            info.flat(),
                            info.resolution(),
                        ),
                    );
                    builder = builder.with_absolute_axis(&setup).unwrap_or(builder);
                }
            }
        }

        match builder.build() {
            Ok(vd) => {
                log::info!("Created virtual device: {}", virt_name);
                virtual_devices.push(vd);
            }
            Err(e) => {
                log::error!("Failed to build virtual device {}: {}", virt_name, e);
                drop(virtual_devices);
                drop(physical_devices);
                running.store(false, Ordering::SeqCst);
                return;
            }
        }
    }

    log::info!("Force mode (Linux): forwarding loop active — {} devices", sorted.len());

    // Step 3: Poll loop — read events from physical devices and forward to virtual devices
    // Use non-blocking reads with short sleep (~1ms) for low latency
    for phys in &mut physical_devices {
        if let Err(e) = phys.set_nonblocking(true) {
            log::warn!("Failed to set non-blocking on {}: {}", phys.name().unwrap_or("?"), e);
        }
    }

    while running.load(Ordering::SeqCst) {
        let mut had_events = false;

        for (i, phys) in physical_devices.iter_mut().enumerate() {
            match phys.fetch_events() {
                Ok(events) => {
                    let events: Vec<InputEvent> = events.collect();
                    if !events.is_empty() {
                        had_events = true;
                        if let Err(e) = virtual_devices[i].emit(&events) {
                            log::warn!("Failed to emit events to virtual device {}: {}", i, e);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No events available — normal for non-blocking
                }
                Err(e) => {
                    log::warn!("Error reading from physical device {}: {}", i, e);
                }
            }
        }

        // Sleep briefly to avoid busy-spinning; ~1ms matches the Windows 1000Hz rate
        if !had_events {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    log::info!("Force mode (Linux): stopping — releasing devices");

    // Step 4: Cleanup — dropping virtual_devices unplugs them, dropping physical_devices
    // releases the EVIOCGRAB. Explicit drop for clarity.
    drop(virtual_devices);
    drop(physical_devices);

    log::info!("Force mode (Linux): cleanup complete");
}

#[cfg(target_os = "macos")]
fn run_force_forwarding(
    running: Arc<AtomicBool>,
    _manager: Arc<dyn PlatformServices>,
    _assignments: Vec<ResolvedAssignment>,
) {
    log::info!("Force mode: stub (macOS)");
    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
