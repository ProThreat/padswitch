use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Watches for game processes and auto-activates matching presets.
pub struct ProcessWatcher {
    running: Arc<AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl ProcessWatcher {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    pub fn start(&mut self, app: AppHandle) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        let handle = std::thread::Builder::new()
            .name("padswitch-process-watcher".into())
            .spawn(move || watcher_loop(running, app))
            .expect("Failed to spawn process watcher thread");

        self.thread_handle = Some(handle);
        log::info!("Process watcher started");
    }

    pub fn stop(&mut self) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        log::info!("Process watcher stopped");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for ProcessWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

// ---------------------------------------------------------------------------
// Watcher loop
// ---------------------------------------------------------------------------

fn watcher_loop(running: Arc<AtomicBool>, app: AppHandle) {
    // Track which game rule is currently active (to avoid re-triggering)
    let mut active_rule_id: Option<String> = None;
    // Profile that was active before the game launched (for reverting)
    let mut pre_game_profile_id: Option<String> = None;

    while running.load(Ordering::SeqCst) {
        let state = app.state::<AppState>();

        // Read game rules and current profile (brief lock)
        let (rules, current_profile_id) = {
            let inner = state.lock_inner();
            (
                inner.config.game_rules.clone(),
                inner.config.settings.active_profile_id.clone(),
            )
        };

        let processes = list_running_processes();

        // Find the first enabled rule that matches a running process
        let matched_rule = rules
            .iter()
            .filter(|r| r.enabled)
            .find(|r| {
                processes
                    .iter()
                    .any(|p| p.eq_ignore_ascii_case(&r.exe_name))
            });

        match (&active_rule_id, matched_rule) {
            (None, Some(rule)) => {
                // Game just launched — activate its profile
                log::info!(
                    "Game detected: {} — activating profile {}",
                    rule.exe_name,
                    rule.profile_id
                );
                if activate_profile_internal(&app, &state, &rule.profile_id) {
                    pre_game_profile_id = current_profile_id;
                    active_rule_id = Some(rule.id.clone());
                }
            }
            (Some(_), None) => {
                // Game exited — revert to previous profile
                log::info!("Game exited — reverting to previous profile");
                active_rule_id = None;

                if let Some(ref prev_id) = pre_game_profile_id {
                    activate_profile_internal(&app, &state, prev_id);
                } else {
                    // No previous profile — clear active and notify frontend
                    let mut inner = state.lock_inner();
                    inner.config.settings.active_profile_id = None;
                    let _ = inner.config.save();
                    drop(inner);
                    crate::tray::rebuild_tray_menu(&app);
                    let _ = app.emit(
                        "profile-activated",
                        serde_json::json!({
                            "profile_id": null,
                            "assignments": [],
                            "routing_mode": "Minimal",
                        }),
                    );
                }
                pre_game_profile_id = None;
            }
            (Some(current_id), Some(rule)) if *current_id != rule.id => {
                // Different game matched — switch to new game's profile
                log::info!(
                    "Game switch: {} — activating profile {}",
                    rule.exe_name,
                    rule.profile_id
                );
                if activate_profile_internal(&app, &state, &rule.profile_id) {
                    active_rule_id = Some(rule.id.clone());
                }
            }
            _ => {
                // No change
            }
        }

        // Poll every 3 seconds
        for _ in 0..30 {
            if !running.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

/// Activate a profile by ID from the watcher thread.
/// If forwarding is currently active, stops and restarts it with the new assignments.
/// Returns `true` if the profile was found and activated, `false` if it doesn't exist.
fn activate_profile_internal(app: &AppHandle, state: &AppState, profile_id: &str) -> bool {
    let manager = state.manager().clone();
    let mut inner = state.lock_inner();

    let profile = match inner.config.profiles.iter().find(|p| p.id == profile_id) {
        Some(p) => p.clone(),
        None => {
            log::warn!("Game rule references unknown profile: {}", profile_id);
            return false;
        }
    };

    inner.config.settings.active_profile_id = Some(profile_id.to_string());
    inner.assignments = profile.assignments.clone();
    let _ = inner.config.save();

    // If forwarding is active, restart the loop with the new profile's assignments
    if inner.forwarding_active {
        log::info!("Forwarding active — restarting with new profile");
        if let Err(e) = inner.restart_forwarding(manager) {
            log::error!("Failed to restart forwarding: {}", e);
            let _ = app.emit(
                "forwarding-status",
                serde_json::json!({ "active": false, "error": e.to_string() }),
            );
        }
    }

    drop(inner);

    crate::tray::rebuild_tray_menu(app);

    let _ = app.emit(
        "profile-activated",
        serde_json::json!({
            "profile_id": profile_id,
            "assignments": profile.assignments,
            "routing_mode": profile.routing_mode,
        }),
    );

    true
}

// ---------------------------------------------------------------------------
// Process listing (platform-specific)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn list_running_processes() -> Vec<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(h) => h,
            Err(e) => {
                log::warn!("CreateToolhelp32Snapshot failed: {}", e);
                return vec![];
            }
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut names = Vec::new();

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
                if !name.is_empty() {
                    names.push(name);
                }

                entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
        names
    }
}

#[cfg(target_os = "linux")]
fn list_running_processes() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
                let comm_path = entry.path().join("comm");
                if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                    let trimmed = comm.trim().to_string();
                    if !trimmed.is_empty() {
                        names.push(trimmed);
                    }
                }
            }
        }
    }
    names
}

#[cfg(target_os = "macos")]
fn list_running_processes() -> Vec<String> {
    vec![]
}
