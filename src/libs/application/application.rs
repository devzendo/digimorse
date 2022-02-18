use log::debug;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use bus::Bus;
use clap::arg_enum;
use syncbox::ScheduledThreadPool;
use crate::libs::keyer_io::keyer_io::KeyingEvent;

arg_enum! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Mode {
        GUI,
        ListAudioDevices,
        SerialDiag,
        KeyerDiag,
        SourceEncoderDiag
    }
}

pub trait BusOutput<T> {
    fn clear_output_tx(&mut self);
    fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<T>>>);
}

// The Application handles all the wiring between the active components of the system. The wiring
// 'loom' is different depending on the mode enum.
// It also holds the termination flag, and system-wide concerns such as PortAudio, the scheduled
// thread pool, etc..
pub struct Application {
    terminate_flag: Arc<AtomicBool>,
    scheduled_thread_pool: Arc<ScheduledThreadPool>,
    mode: Option<Mode>,

    keying_event_bus: Option<Bus<KeyingEvent>>,
}

impl Application {
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = Some(mode);
        // TODO far more to do here, set up wiring for each Mode
        match mode {
            Mode::KeyerDiag => {
                self.keying_event_bus = Some(Bus::new(16));
            }
            Mode::SourceEncoderDiag => {}
            _ => {}
        }
    }

    pub fn get_mode(&self) -> Option<Mode> {
        return self.mode.clone();
    }
}

impl Application {
    pub fn new(terminate_flag: Arc<AtomicBool>,
               scheduled_thread_pool: Arc<ScheduledThreadPool>,
    ) -> Self {
        debug!("Constructing Application");

        Self {
            terminate_flag,
            scheduled_thread_pool,
            mode: None,

            keying_event_bus: None,
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