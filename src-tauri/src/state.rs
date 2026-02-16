use crate::config::{AppConfig, Profile, RoutingMode};
use crate::device::{DriverStatus, PhysicalDevice, SlotAssignment};
use crate::input_loop::{InputLoop, ResolvedAssignment};
use crate::platform::PlatformServices;
use crate::process_watcher::ProcessWatcher;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct Inner {
    pub devices: Vec<PhysicalDevice>,
    pub assignments: Vec<SlotAssignment>,
    pub driver_status: DriverStatus,
    pub forwarding_active: bool,
    pub config: AppConfig,
    pub input_loop: InputLoop,
}

impl Inner {
    /// Get the currently active profile, if any.
    pub fn active_profile(&self) -> Option<&Profile> {
        let active_id = self.config.settings.active_profile_id.as_deref()?;
        self.config.profiles.iter().find(|p| p.id == active_id)
    }

    /// Get the routing mode of the active profile (defaults to Minimal).
    pub fn active_routing_mode(&self) -> RoutingMode {
        self.active_profile()
            .map(|p| p.routing_mode.clone())
            .unwrap_or_default()
    }

    /// Resolve enabled assignments to ResolvedAssignments by looking up real device data.
    /// Returns only assignments whose device_id matches a known device.
    pub fn resolve_assignments(&self) -> Vec<ResolvedAssignment> {
        self.assignments
            .iter()
            .filter(|a| a.enabled)
            .filter_map(|a| {
                let device = self.devices.iter().find(|d| d.id == a.device_id)?;
                Some(ResolvedAssignment {
                    instance_path: device.instance_path.clone(),
                    xinput_slot: device.xinput_slot,
                    target_slot: a.slot,
                })
            })
            .collect()
    }

    /// Start forwarding with current assignments and active routing mode.
    /// Runs preflight checks (elevation for Minimal, drivers for Force),
    /// resolves assignments, and starts the input loop.
    pub fn start_forwarding(
        &mut self,
        manager: Arc<dyn PlatformServices>,
    ) -> crate::error::Result<()> {
        if self.forwarding_active {
            return Ok(());
        }

        let mode = self.active_routing_mode();
        self.preflight_check(&mode, &*manager)?;

        let resolved = self.resolve_assignments();
        if resolved.is_empty() {
            return Err(crate::error::PadSwitchError::Forwarding(
                "No valid device assignments to forward".into(),
            ));
        }

        log::info!(
            "Starting forwarding ({:?} mode) with {} resolved assignments",
            mode,
            resolved.len()
        );

        self.input_loop.start(manager, resolved, mode)?;
        self.forwarding_active = true;
        Ok(())
    }

    /// Stop forwarding and clean up.
    pub fn stop_forwarding(&mut self) {
        if !self.forwarding_active {
            return;
        }
        self.input_loop.stop();
        self.forwarding_active = false;
    }

    /// Restart forwarding (stop + start). Used when switching profiles while active.
    /// If start fails, forwarding stays stopped and the error is returned.
    pub fn restart_forwarding(
        &mut self,
        manager: Arc<dyn PlatformServices>,
    ) -> crate::error::Result<()> {
        self.stop_forwarding();
        self.start_forwarding(manager)
    }

    /// Run preflight checks for a given routing mode.
    fn preflight_check(
        &self,
        mode: &RoutingMode,
        manager: &dyn PlatformServices,
    ) -> crate::error::Result<()> {
        match mode {
            RoutingMode::Minimal => {
                if !crate::platform::is_elevated() {
                    return Err(crate::error::PadSwitchError::Platform(
                        "Minimal mode requires administrator privileges. Restart PadSwitch as Administrator.".into(),
                    ));
                }
            }
            RoutingMode::Force => {
                let drivers = manager.check_drivers()?;
                if !drivers.hidhide_installed {
                    return Err(crate::error::PadSwitchError::DriverNotInstalled(
                        "HidHide is required for Force mode. Install it from github.com/nefarius/HidHide/releases".into(),
                    ));
                }
                if !drivers.vigembus_installed {
                    return Err(crate::error::PadSwitchError::DriverNotInstalled(
                        "ViGEmBus is required for Force mode. Install it from github.com/nefarius/ViGEmBus/releases".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

pub struct AppState {
    inner: Mutex<Inner>,
    manager: Arc<dyn PlatformServices>,
    /// Process watcher has its own lock to avoid contention with inner.
    watcher: Mutex<ProcessWatcher>,
}

impl AppState {
    pub fn new(manager: Arc<dyn PlatformServices>) -> Self {
        let config = AppConfig::load().unwrap_or_default();
        Self {
            inner: Mutex::new(Inner {
                devices: vec![],
                assignments: vec![],
                driver_status: DriverStatus::default(),
                forwarding_active: false,
                config,
                input_loop: InputLoop::new(),
            }),
            manager,
            watcher: Mutex::new(ProcessWatcher::new()),
        }
    }

    pub fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap()
    }

    pub fn manager(&self) -> &Arc<dyn PlatformServices> {
        &self.manager
    }

    pub fn lock_watcher(&self) -> MutexGuard<'_, ProcessWatcher> {
        self.watcher.lock().unwrap()
    }
}
