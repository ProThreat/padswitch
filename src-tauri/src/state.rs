use crate::config::AppConfig;
use crate::device::{DriverStatus, PhysicalDevice, SlotAssignment};
use std::sync::Mutex;

pub struct AppState {
    pub devices: Mutex<Vec<PhysicalDevice>>,
    pub assignments: Mutex<Vec<SlotAssignment>>,
    pub driver_status: Mutex<DriverStatus>,
    pub forwarding_active: Mutex<bool>,
    pub config: Mutex<AppConfig>,
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        Self {
            devices: Mutex::new(vec![]),
            assignments: Mutex::new(vec![]),
            driver_status: Mutex::new(DriverStatus::default()),
            forwarding_active: Mutex::new(false),
            config: Mutex::new(config),
        }
    }
}
