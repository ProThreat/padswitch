mod commands;
mod config;
mod device;
mod error;
mod hidhide;
mod input_loop;
mod platform;
mod process_watcher;
mod setupdi;
mod state;
mod tray;
mod vigem;

use state::AppState;
use tauri::Manager;

/// Path to the lockfile used to detect dirty shutdowns.
fn lockfile_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("padswitch").join("padswitch.lock"))
}

/// Check if previous session ended dirty (lockfile exists) and perform recovery reset.
fn check_dirty_shutdown(app: &tauri::AppHandle) {
    let Some(path) = lockfile_path() else { return };
    if !path.exists() {
        return;
    }
    log::warn!("Dirty shutdown detected — running automatic reset");

    // Remove stale lockfile first
    let _ = std::fs::remove_file(&path);

    // Re-enable and unhide all devices that were known last session.
    // We need to enumerate fresh devices since state.devices is empty at startup.
    let state = app.state::<AppState>();
    let manager = state.manager().clone();

    // Try to enumerate current devices and re-enable/unhide each
    if let Ok(devices) = manager.enumerate_devices() {
        for dev in &devices {
            let _ = manager.enable_device(&dev.instance_path);
            let _ = manager.unhide_device(&dev.instance_path);
        }
    }

    // Deactivate HidHide globally
    let _ = manager.deactivate_hiding();

    // Clear active profile (it may reference a state that was mid-operation)
    let mut inner = state.lock_inner();
    inner.config.settings.active_profile_id = None;
    let _ = inner.config.save();

    log::info!("Dirty shutdown recovery complete");
}

/// Create the lockfile (marks session as "in progress").
fn create_lockfile() {
    if let Some(path) = lockfile_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, std::process::id().to_string());
    }
}

/// Remove the lockfile (marks clean shutdown).
fn remove_lockfile() {
    if let Some(path) = lockfile_path() {
        let _ = std::fs::remove_file(&path);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let manager = platform::create_platform();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new(manager))
        .invoke_handler(tauri::generate_handler![
            commands::get_connected_devices,
            commands::check_driver_status,
            commands::toggle_device,
            commands::apply_assignments,
            commands::start_forwarding,
            commands::stop_forwarding,
            commands::is_forwarding,
            commands::get_profiles,
            commands::save_profile,
            commands::delete_profile,
            commands::activate_profile,
            commands::is_elevated,
            commands::detect_xinput_slot,
            commands::confirm_device_slot,
            commands::get_game_rules,
            commands::add_game_rule,
            commands::delete_game_rule,
            commands::toggle_game_rule,
            commands::start_process_watcher,
            commands::stop_process_watcher,
            commands::is_watcher_running,
            commands::reset_all,
            commands::get_settings,
            commands::update_settings,
        ])
        .setup(|app| {
            tray::setup_tray(app.handle())?;

            // Detect and recover from dirty shutdown (crash while devices were modified)
            check_dirty_shutdown(app.handle());

            // Mark this session as active
            create_lockfile();

            // Auto-start process watcher if enabled in settings
            let state = app.state::<AppState>();
            let auto_switch = state.lock_inner().config.settings.auto_switch;
            if auto_switch {
                state.lock_watcher().start(app.handle().clone());
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                remove_lockfile();
            }
        });
}
