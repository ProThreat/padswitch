use crate::config::{Profile, RoutingMode, Settings};
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

    // Read the instance_path while holding the lock briefly
    let instance_path = {
        let inner = state.lock_inner();
        let device = inner
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| crate::error::PadSwitchError::DeviceNotFound(device_id.clone()))?;
        device.instance_path.clone()
    };

    // Call platform I/O without holding the lock
    if hidden {
        manager.hide_device(&instance_path)?;
    } else {
        manager.unhide_device(&instance_path)?;
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
    if inner.forwarding_active {
        return Ok(());
    }

    let assignments = inner.assignments.clone();
    let mode = inner
        .config
        .profiles
        .iter()
        .find(|p| inner.config.settings.active_profile_id.as_deref() == Some(&p.id))
        .map(|p| p.routing_mode.clone())
        .unwrap_or_default();

    inner.input_loop.start(manager, assignments, mode)?;
    inner.forwarding_active = true;
    drop(inner);

    let _ = app.emit("forwarding-status", serde_json::json!({ "active": true }));
    Ok(())
}

#[tauri::command]
pub fn stop_forwarding(app: AppHandle, state: State<AppState>) -> Result<()> {
    let mut inner = state.lock_inner();
    if !inner.forwarding_active {
        return Ok(());
    }
    inner.input_loop.stop();
    inner.forwarding_active = false;
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
