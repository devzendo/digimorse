#[macro_use]

use log::debug;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use clap::arg_enum;
use syncbox::ScheduledThreadPool;

arg_enum! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Mode {
        GUI,
        ListAudioDevices,
        SerialDiag,
        KeyerDiag,
        SourceEncoderDiag
    }
}


// The Application handles all the wiring between the active components of the system. The wiring
// 'loom' is different depending on the mode enum.
// It also holds the termination flag, and system-wide concerns such as PortAudio, the scheduled
// thread pool, etc..
pub struct Application {
    terminate_flag: Arc<AtomicBool>,
    scheduled_thread_pool: Arc<ScheduledThreadPool>,
    mode: Option<Mode>,
}

impl Application {
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = Some(mode);
        // TODO far more to do here, set up wiring for each Mode
    }

    pub fn get_mode(&self) -> Option<Mode> {
        return self.mode.clone();
    }
}

impl Application {
    fn new(terminate_flag: Arc<AtomicBool>,
           scheduled_thread_pool: Arc<ScheduledThreadPool>,
    ) -> Self {
        debug!("Constructing Application");

        Self {
            terminate_flag,
            scheduled_thread_pool,
            mode: None,
        }
    }

    // Setting the terminate AtomicBool will allow the thread to stop on its own.
    pub fn terminate(&mut self) {
        debug!("Terminating Application");
        self.terminate_flag.store(true, core::sync::atomic::Ordering::SeqCst);
        debug!("Terminated Application");
    }

    // Has the Application been terminated
    pub fn terminated(&self) -> bool {
        debug!("Is Application terminated?");
        let ret = self.terminate_flag.load(core::sync::atomic::Ordering::SeqCst);
        debug!("Termination state is {}", ret);
        ret
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        debug!("Application signalling termination on drop");
        self.terminate();
    }
}

#[cfg(test)]
#[path = "./application_spec.rs"]
mod application_spec;