use crate::device::SlotAssignment;
use crate::platform::ControllerManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Manages the input forwarding loop.
///
/// Runs on a dedicated `std::thread` (NOT tokio) for consistent sub-ms timing.
/// Polls at ~1000Hz: reads hidden physical devices, writes to virtual controllers.
pub struct InputLoop {
    running: Arc<AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl InputLoop {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }

    /// Start the forwarding loop with the given assignments.
    pub fn start(
        &mut self,
        _manager: Arc<dyn ControllerManager>,
        _assignments: Vec<SlotAssignment>,
    ) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        let handle = std::thread::Builder::new()
            .name("padswitch-input-loop".into())
            .spawn(move || {
                log::info!("Input forwarding loop started");

                while running.load(Ordering::SeqCst) {
                    // TODO: For each assignment:
                    // 1. Read physical device state via manager.read_gamepad_state()
                    // 2. Write to virtual controller via manager.write_virtual_state()

                    // Sleep ~1ms for ~1000Hz polling
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }

                log::info!("Input forwarding loop stopped");
            })
            .expect("Failed to spawn input loop thread");

        self.thread_handle = Some(handle);
    }

    /// Stop the forwarding loop.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for InputLoop {
    fn drop(&mut self) {
        self.stop();
    }
}
