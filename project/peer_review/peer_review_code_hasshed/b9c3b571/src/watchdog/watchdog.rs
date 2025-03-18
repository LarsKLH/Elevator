use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

pub struct Watchdog {
    duration: Duration,
    reset_signal: Arc<Mutex<Option<Arc<Notify>>>>, // Changed to Arc<Notify>
}

impl Watchdog {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            reset_signal: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_reset_handle(&self) -> ResetHandle {
        ResetHandle {
            reset_signal: self.reset_signal.clone(),
        }
    }

    pub async fn await_timeout(self) {
        loop {
            // Create a new `Notify` instance and store it in the mutex
            let reset_notifier = {
                let mut guard = self.reset_signal.lock().unwrap();
                let notify = Arc::new(Notify::new()); // Use Arc for shared ownership
                *guard = Some(notify.clone()); // Clone the Arc to store it in the mutex
                notify // Return the cloned Arc
            };

            // Wait for either the timeout or the reset signal
            tokio::select! {
                _ = sleep(self.duration) => {
                    println!("Watchdog timed out!");
                    break;
                }
                _ = reset_notifier.notified() => {
                    println!("Watchdog reset!");
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ResetHandle {
    reset_signal: Arc<Mutex<Option<Arc<Notify>>>>, // Changed to Arc<Notify>
}

impl ResetHandle {
    pub fn reset(&self) {
        if let Some(notifier) = self.reset_signal.lock().unwrap().take() {
            notifier.notify_one();
        }
    }
}
