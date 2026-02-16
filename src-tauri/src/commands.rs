use crate::config::{GameRule, Profile, RoutingMode, Settings};
use crate::device::{DriverStatus, PhysicalDevice, SlotAssignment};
use crate::error::Result;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[tauri::command]
pub fn get_connected_devices(state: State<AppState>) -> Result<Vec<PhysicalDevice>> {
    let manager = state.manager().clone();
    let devices = manager.enumerate_devices()?;
    let mut inner = state.lock_inner();
    inner.devices = devices.clone();
    Ok(devices)
}

#[tauri::command]
pub fn check_driver_status(state: State<AppState>) -> Result<DriverStatus> {
    let manager = state.manager().clone();
    let status = manager.check_drivers()?;
    let mut inner = state.lock_inner();
    inner.driver_status = status.clone();
    Ok(status)
}

#[tauri::command]
pub fn toggle_device(state: State<AppState>, device_id: String, hidden: bool) -> Result<()> {
    let manager = state.manager().clone();

    // Read the instance_path and active routing mode while holding the lock briefly
    let (instance_path, mode) = {
        let inner = state.lock_inner();
        let device = inner
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| crate::error::PadSwitchError::DeviceNotFound(device_id.clone()))?;
        (device.instance_path.clone(), inner.active_routing_mode())
    };

    // Call platform I/O without holding the lock.
    // Minimal mode: use SetupDi disable/enable (OS-level, no third-party drivers).
    // Force mode: use HidHide hide/unhide (filter driver).
    match mode {
        RoutingMode::Minimal => {
            if hidden {
                manager.disable_device(&instance_path)?;
            } else {
                manager.enable_device(&instance_path)?;
            }
        }
        RoutingMode::Force => {
            if hidden {
                manager.hide_device(&instance_path)?;
            } else {
                manager.unhide_device(&instance_path)?;
            }
        }
    }

    // Re-lock to write result
    let mut inner = state.lock_inner();
    if let Some(device) = inner.devices.iter_mut().find(|d| d.id == device_id) {
        device.hidden = hidden;
    }
    Ok(())
}

#[tauri::command]
pub fn apply_assignments(state: State<AppState>, assignments: Vec<SlotAssignment>) -> Result<()> {
    let mut inner = state.lock_inner();
    inner.assignments = assignments;
    Ok(())
}

#[tauri::command]
pub fn start_forwarding(app: AppHandle, state: State<AppState>) -> Result<()> {
    let manager = state.manager().clone();
    let mut inner = state.lock_inner();

    inner.start_forwarding(manager)?;
    drop(inner);

    let _ = app.emit("forwarding-status", serde_json::json!({ "active": true }));
    Ok(())
}

#[tauri::command]
pub fn stop_forwarding(app: AppHandle, state: State<AppState>) -> Result<()> {
    let mut inner = state.lock_inner();
    inner.stop_forwarding();
    drop(inner);

    let _ = app.emit("forwarding-status", serde_json::json!({ "active": false }));
    Ok(())
}

#[tauri::command]
pub fn is_forwarding(state: State<AppState>) -> bool {
    state.lock_inner().forwarding_active
}

// --- Profile commands ---

#[tauri::command]
pub fn get_profiles(state: State<AppState>) -> Result<Vec<Profile>> {
    let inner = state.lock_inner();
    Ok(inner.config.profiles.clone())
}

#[tauri::command]
pub fn save_profile(
    app: AppHandle,
    state: State<AppState>,
    name: String,
    assignments: Vec<SlotAssignment>,
    routing_mode: Option<RoutingMode>,
) -> Result<Profile> {
    let mut inner = state.lock_inner();
    let profile = Profile {
        id: Uuid::new_v4().to_string(),
        name,
        assignments,
        routing_mode: routing_mode.unwrap_or_default(),
    };
    inner.config.profiles.push(profile.clone());
    inner.config.save()?;
    drop(inner);
    crate::tray::rebuild_tray_menu(&app);
    Ok(profile)
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, state: State<AppState>, profile_id: String) -> Result<()> {
    let mut inner = state.lock_inner();
    inner.config.profiles.retain(|p| p.id != profile_id);
    // Clean up game rules that reference this profile
    inner.config.game_rules.retain(|r| r.profile_id != profile_id);
    if inner.config.settings.active_profile_id.as_deref() == Some(&profile_id) {
        inner.config.settings.active_profile_id = None;
    }
    inner.config.save()?;
    drop(inner);
    crate::tray::rebuild_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub fn activate_profile(
    app: AppHandle,
    state: State<AppState>,
    profile_id: String,
) -> Result<Vec<SlotAssignment>> {
    let mut inner = state.lock_inner();
    let profile = inner
        .config
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| crate::error::PadSwitchError::Config("Profile not found".into()))?
        .clone();

    inner.config.settings.active_profile_id = Some(profile_id);
    inner.config.save()?;
    inner.assignments = profile.assignments.clone();
    drop(inner);
    crate::tray::rebuild_tray_menu(&app);

    Ok(profile.assignments)
}

// --- Reset command ---

/// Nuclear reset: stop everything, re-enable all devices, unhide all devices,
/// deactivate HidHide, clear active profile. Use when controllers stop working.
#[tauri::command]
pub fn reset_all(app: AppHandle, state: State<AppState>) -> Result<()> {
    log::info!("Reset all: starting full reset");

    // 1. Stop process watcher
    {
        let mut watcher = state.lock_watcher();
        watcher.stop();
    }

    // 2. Stop forwarding (input loop handles its own cleanup for the current mode)
    let manager = state.manager().clone();
    {
        let mut inner = state.lock_inner();
        inner.stop_forwarding();
    }

    // 3. Re-enable and unhide all known devices (both are idempotent, errors swallowed)
    let device_paths: Vec<String> = {
        let inner = state.lock_inner();
        inner.devices.iter().map(|d| d.instance_path.clone()).collect()
    };

    for path in &device_paths {
        if let Err(e) = manager.enable_device(path) {
            log::warn!("Reset: enable_device failed for {}: {}", path, e);
        }
        if let Err(e) = manager.unhide_device(path) {
            log::warn!("Reset: unhide_device failed for {}: {}", path, e);
        }
    }

    // 4. Deactivate HidHide globally
    if let Err(e) = manager.deactivate_hiding() {
        log::warn!("Reset: deactivate_hiding failed: {}", e);
    }

    // 5. Clear active profile
    {
        let mut inner = state.lock_inner();
        inner.config.settings.active_profile_id = None;
        inner.assignments.clear();
        let _ = inner.config.save();
    }

    // 6. Notify frontend
    let _ = app.emit("forwarding-status", serde_json::json!({ "active": false }));
    let _ = app.emit(
        "profile-activated",
        serde_json::json!({
            "profile_id": null,
            "assignments": [],
            "routing_mode": "Minimal",
        }),
    );

    crate::tray::rebuild_tray_menu(&app);

    log::info!("Reset all: complete");
    Ok(())
}

// --- Environment commands ---

#[tauri::command]
pub fn is_elevated() -> bool {
    crate::platform::is_elevated()
}

/// Poll all XInput slots for a button press. Returns the slot number (0-3) that
/// first receives input, or null if no input within ~5 seconds.
/// Used by the "Identify" feature to reliably map physical device → XInput slot.
#[tauri::command]
pub fn detect_xinput_slot(state: State<AppState>) -> Result<Option<u32>> {
    let manager = state.manager().clone();

    // Snapshot current button state for all 4 slots
    let mut baseline = [0u16; 4];
    for slot in 0..4u32 {
        if let Ok(gs) = manager.read_gamepad_state(&slot.to_string()) {
            baseline[slot as usize] = gs.buttons;
        }
    }

    // Poll at ~60Hz for up to 5 seconds
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        for slot in 0..4u32 {
            if let Ok(gs) = manager.read_gamepad_state(&slot.to_string()) {
                // Detect any new button press (bits that weren't set before)
                if gs.buttons & !baseline[slot as usize] != 0 {
                    log::info!("Detected button press on XInput slot {}", slot);
                    return Ok(Some(slot));
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(None) // Timeout — no input detected
}

/// Update a device's XInput slot assignment after identification.
#[tauri::command]
pub fn confirm_device_slot(
    state: State<AppState>,
    device_id: String,
    xinput_slot: u32,
) -> Result<()> {
    let mut inner = state.lock_inner();
    if let Some(device) = inner.devices.iter_mut().find(|d| d.id == device_id) {
        log::info!(
            "Confirmed {} ({}) -> XInput slot {}",
            device.name,
            device.id,
            xinput_slot
        );
        device.xinput_slot = Some(xinput_slot);
    }
    Ok(())
}

// --- Game rule commands ---

#[tauri::command]
pub fn get_game_rules(state: State<AppState>) -> Result<Vec<GameRule>> {
    let inner = state.lock_inner();
    Ok(inner.config.game_rules.clone())
}

#[tauri::command]
pub fn add_game_rule(
    state: State<AppState>,
    exe_name: String,
    profile_id: String,
) -> Result<GameRule> {
    let mut inner = state.lock_inner();
    // Validate that the referenced profile exists
    if !inner.config.profiles.iter().any(|p| p.id == profile_id) {
        return Err(crate::error::PadSwitchError::Config(
            format!("Profile '{}' does not exist", profile_id),
        ));
    }
    let rule = GameRule {
        id: Uuid::new_v4().to_string(),
        exe_name,
        profile_id,
        enabled: true,
    };
    inner.config.game_rules.push(rule.clone());
    inner.config.save()?;
    Ok(rule)
}

#[tauri::command]
pub fn delete_game_rule(state: State<AppState>, rule_id: String) -> Result<()> {
    let mut inner = state.lock_inner();
    inner.config.game_rules.retain(|r| r.id != rule_id);
    inner.config.save()?;
    Ok(())
}

#[tauri::command]
pub fn toggle_game_rule(state: State<AppState>, rule_id: String, enabled: bool) -> Result<()> {
    let mut inner = state.lock_inner();
    if let Some(rule) = inner.config.game_rules.iter_mut().find(|r| r.id == rule_id) {
        rule.enabled = enabled;
    }
    inner.config.save()?;
    Ok(())
}

// --- Process watcher commands ---

#[tauri::command]
pub fn start_process_watcher(app: AppHandle, state: State<AppState>) -> Result<()> {
    let mut watcher = state.lock_watcher();
    watcher.start(app);
    Ok(())
}

#[tauri::command]
pub fn stop_process_watcher(state: State<AppState>) -> Result<()> {
    let mut watcher = state.lock_watcher();
    watcher.stop();
    Ok(())
}

#[tauri::command]
pub fn is_watcher_running(state: State<AppState>) -> bool {
    state.lock_watcher().is_running()
}

// --- Settings commands ---

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings> {
    let inner = state.lock_inner();
    Ok(inner.config.settings.clone())
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: Settings) -> Result<()> {
    let mut inner = state.lock_inner();
    inner.config.settings = settings;
    inner.config.save()?;
    Ok(())
}
