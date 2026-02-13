use crate::config::AppConfig;
use crate::device::{DriverStatus, PhysicalDevice, SlotAssignment};
use crate::input_loop::InputLoop;
use crate::platform::PlatformServices;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct Inner {
    pub devices: Vec<PhysicalDevice>,
    pub assignments: Vec<SlotAssignment>,
    pub driver_status: DriverStatus,
    pub forwarding_active: bool,
    pub config: AppConfig,
    pub input_loop: InputLoop,
}

pub struct AppState {
    inner: Mutex<Inner>,
    manager: Arc<dyn PlatformServices>,
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
        }
    }

    pub fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap()
    }

    pub fn manager(&self) -> &Arc<dyn PlatformServices> {
        &self.manager
    }
}
