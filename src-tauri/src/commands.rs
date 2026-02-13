use crate::config::{Profile, Settings};
use crate::device::{DriverStatus, PhysicalDevice, SlotAssignment};
use crate::error::Result;
use crate::platform;
use crate::state::AppState;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_connected_devices(state: State<AppState>) -> Result<Vec<PhysicalDevice>> {
    let manager = platform::create_manager();
    let devices = manager.enumerate_devices()?;
    let mut stored = state.devices.lock().unwrap();
    *stored = devices.clone();
    Ok(devices)
}

#[tauri::command]
pub fn check_driver_status(state: State<AppState>) -> Result<DriverStatus> {
    let manager = platform::create_manager();
    let status = manager.check_drivers()?;
    let mut stored = state.driver_status.lock().unwrap();
    *stored = status.clone();
    Ok(status)
}

#[tauri::command]
pub fn toggle_device(state: State<AppState>, device_id: String, hidden: bool) -> Result<()> {
    let manager = platform::create_manager();
    let mut devices = state.devices.lock().unwrap();
    let device = devices
        .iter_mut()
        .find(|d| d.id == device_id)
        .ok_or_else(|| crate::error::PadSwitchError::DeviceNotFound(device_id.clone()))?;

    if hidden {
        manager.hide_device(&device.instance_path)?;
    } else {
        manager.unhide_device(&device.instance_path)?;
    }

    device.hidden = hidden;
    Ok(())
}

#[tauri::command]
pub fn apply_assignments(state: State<AppState>, assignments: Vec<SlotAssignment>) -> Result<()> {
    let mut stored = state.assignments.lock().unwrap();
    *stored = assignments;
    Ok(())
}

#[tauri::command]
pub fn start_forwarding(state: State<AppState>) -> Result<()> {
    let mut active = state.forwarding_active.lock().unwrap();
    if *active {
        return Ok(());
    }
    // TODO: Actually start the input loop
    // 1. Hide all assigned devices via HidHide
    // 2. Create virtual controllers via ViGEmBus (in slot order)
    // 3. Start the input forwarding thread
    *active = true;
    Ok(())
}

#[tauri::command]
pub fn stop_forwarding(state: State<AppState>) -> Result<()> {
    let mut active = state.forwarding_active.lock().unwrap();
    if !*active {
        return Ok(());
    }
    // TODO: Actually stop the input loop
    // 1. Stop the forwarding thread
    // 2. Destroy virtual controllers
    // 3. Unhide all devices
    *active = false;
    Ok(())
}

#[tauri::command]
pub fn is_forwarding(state: State<AppState>) -> bool {
    *state.forwarding_active.lock().unwrap()
}

// --- Profile commands ---

#[tauri::command]
pub fn get_profiles(state: State<AppState>) -> Result<Vec<Profile>> {
    let config = state.config.lock().unwrap();
    Ok(config.profiles.clone())
}

#[tauri::command]
pub fn save_profile(
    state: State<AppState>,
    name: String,
    assignments: Vec<SlotAssignment>,
) -> Result<Profile> {
    let mut config = state.config.lock().unwrap();
    let profile = Profile {
        id: Uuid::new_v4().to_string(),
        name,
        assignments,
    };
    config.profiles.push(profile.clone());
    config.save()?;
    Ok(profile)
}

#[tauri::command]
pub fn delete_profile(state: State<AppState>, profile_id: String) -> Result<()> {
    let mut config = state.config.lock().unwrap();
    config.profiles.retain(|p| p.id != profile_id);
    if config.settings.active_profile_id.as_deref() == Some(&profile_id) {
        config.settings.active_profile_id = None;
    }
    config.save()?;
    Ok(())
}

#[tauri::command]
pub fn activate_profile(state: State<AppState>, profile_id: String) -> Result<Vec<SlotAssignment>> {
    let mut config = state.config.lock().unwrap();
    let profile = config
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| crate::error::PadSwitchError::Config("Profile not found".into()))?
        .clone();

    config.settings.active_profile_id = Some(profile_id);
    config.save()?;

    let mut assignments = state.assignments.lock().unwrap();
    *assignments = profile.assignments.clone();

    Ok(profile.assignments)
}

// --- Settings commands ---

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings> {
    let config = state.config.lock().unwrap();
    Ok(config.settings.clone())
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: Settings) -> Result<()> {
    let mut config = state.config.lock().unwrap();
    config.settings = settings;
    config.save()?;
    Ok(())
}
