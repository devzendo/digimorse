use log::debug;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use syncbox::ScheduledThreadPool;

// The Application handles all the wiring between the active components of the system. The wiring
// 'loom' is different depending on the mode enum.
// It also holds the termination flag, and system-wide concerns such as PortAudio, the scheduled
// thread pool, etc..
pub struct Application {
    terminate_flag: Arc<AtomicBool>,
    scheduled_thread_pool: Arc<ScheduledThreadPool>,
}

impl Application {
    fn new(terminate_flag: Arc<AtomicBool>,
           scheduled_thread_pool: Arc<ScheduledThreadPool>,
    ) -> Self {
        debug!("Constructing Application");

        Self {
            terminate_flag,
            scheduled_thread_pool,
        }
    }

}

#[cfg(test)]
#[path = "./application_spec.rs"]
mod application_spec;