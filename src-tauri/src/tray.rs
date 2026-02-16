use crate::state::AppState;
use tauri::{
    menu::{Menu, MenuBuilder, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

const PROFILE_PREFIX: &str = "profile:";

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_tray_menu(app)?;

    TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("PadSwitch")
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            if let Some(profile_id) = id.strip_prefix(PROFILE_PREFIX) {
                activate_profile_from_tray(app, profile_id);
            } else {
                match id {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "reset" => {
                        reset_from_tray(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Rebuild the tray menu (call after profile changes).
pub fn rebuild_tray_menu(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_tray_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let mut builder = MenuBuilder::new(app);

    // Profiles submenu (only if profiles exist)
    let state: Option<tauri::State<'_, AppState>> = app.try_state();
    if let Some(state) = state {
        let inner = state.lock_inner();
        let profiles = &inner.config.profiles;
        let active_id = inner.config.settings.active_profile_id.as_deref();

        if !profiles.is_empty() {
            let mut submenu_items: Vec<MenuItem<tauri::Wry>> = Vec::new();
            for profile in profiles {
                let label = if active_id == Some(&profile.id) {
                    format!("* {}", profile.name)
                } else {
                    profile.name.clone()
                };
                let item_id = format!("{}{}", PROFILE_PREFIX, profile.id);
                let item = MenuItem::with_id(app, item_id, label, true, None::<&str>)?;
                submenu_items.push(item);
            }

            let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
                submenu_items.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
            let submenu = Submenu::with_items(app, "Profiles", true, &refs)?;
            builder = builder.item(&submenu);
            builder = builder.item(&PredefinedMenuItem::separator(app)?);
        }
    }

    let show = MenuItem::with_id(app, "show", "Show PadSwitch", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "Reset All Controllers", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    builder
        .item(&show)
        .item(&reset)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&quit)
        .build()
}

fn reset_from_tray(app: &AppHandle) {
    let state: Option<tauri::State<'_, AppState>> = app.try_state();
    let Some(state) = state else { return };

    // Stop watcher
    state.lock_watcher().stop();

    // Stop forwarding
    let manager = state.manager().clone();
    {
        let mut inner = state.lock_inner();
        if inner.forwarding_active {
            inner.input_loop.stop();
            inner.forwarding_active = false;
        }
    }

    // Re-enable and unhide all known devices
    let device_paths: Vec<String> = {
        let inner = state.lock_inner();
        inner.devices.iter().map(|d| d.instance_path.clone()).collect()
    };
    for path in &device_paths {
        let _ = manager.enable_device(path);
        let _ = manager.unhide_device(path);
    }
    let _ = manager.deactivate_hiding();

    // Clear active profile
    {
        let mut inner = state.lock_inner();
        inner.config.settings.active_profile_id = None;
        inner.assignments.clear();
        let _ = inner.config.save();
    }

    let _ = app.emit("forwarding-status", serde_json::json!({ "active": false }));
    let _ = app.emit(
        "profile-activated",
        serde_json::json!({
            "profile_id": null,
            "assignments": [],
            "routing_mode": "Minimal",
        }),
    );

    rebuild_tray_menu(app);
    log::info!("Reset all from tray: complete");
}

fn activate_profile_from_tray(app: &AppHandle, profile_id: &str) {
    let state: Option<tauri::State<'_, AppState>> = app.try_state();
    let Some(state) = state else { return };

    let result = {
        let mut inner = state.lock_inner();
        let profile = inner
            .config
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned();

        match profile {
            Some(profile) => {
                inner.config.settings.active_profile_id = Some(profile_id.to_string());
                let _ = inner.config.save();
                inner.assignments = profile.assignments.clone();
                Some(profile)
            }
            None => None,
        }
    };

    if let Some(profile) = result {
        let _ = app.emit(
            "profile-activated",
            serde_json::json!({
                "profile_id": profile.id,
                "assignments": profile.assignments,
                "routing_mode": profile.routing_mode,
            }),
        );
        rebuild_tray_menu(app);
    }
}
