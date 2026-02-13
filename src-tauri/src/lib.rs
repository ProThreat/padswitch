mod commands;
mod config;
mod device;
mod error;
mod hidhide;
mod input_loop;
mod platform;
mod state;
mod tray;
mod vigem;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
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
            commands::get_settings,
            commands::update_settings,
        ])
        .setup(|app| {
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
