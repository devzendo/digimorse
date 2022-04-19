extern crate portaudio;

use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::borrow::Borrow;
use std::error::Error;

use bus::{Bus, BusReader};
use clap::arg_enum;
use log::{debug, info};
use portaudio::{InputStreamSettings, OutputStreamSettings, PortAudio};
use syncbox::ScheduledThreadPool;

use crate::libs::audio::tone_generator::KeyingEventToneChannel;
use crate::libs::keyer_io::keyer_io::KeyingEvent;
use crate::libs::transform_bus::transform_bus::TransformBus;
use crate::libs::audio::audio_devices::{open_input_audio_device, open_output_audio_device};

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

pub trait BusInput<T> {
    fn clear_input_rx(&mut self);
    fn set_input_rx(&mut self, input_tx: Arc<Mutex<BusReader<T>>>);
}

fn add_sidetone_channel_to_keying_event(keying_event: KeyingEvent) -> KeyingEventToneChannel {
    return KeyingEventToneChannel { keying_event, tone_channel: 0 };
}

// The Application handles all the wiring between the active components of the system. The wiring
// 'loom' is different depending on the mode enum.
// It also holds the termination flag, and system-wide concerns such as PortAudio, the scheduled
// thread pool, etc..
pub struct Application {
    terminate_flag: Arc<AtomicBool>,
    _scheduled_thread_pool: Arc<ScheduledThreadPool>,
    pa: PortAudio,
    mode: Option<Mode>,

    keyer: Option<Arc<Mutex<dyn BusOutput<KeyingEvent>>>>,
    keying_event_bus: Option<Arc<Mutex<Bus<KeyingEvent>>>>,
    tone_generator: Option<Arc<Mutex<dyn BusInput<KeyingEventToneChannel>>>>,
    keying_event_tone_channel_bus: Option<Arc<Mutex<Bus<KeyingEventToneChannel>>>>,
    keying_event_tone_channel_rx: Option<Arc<Mutex<BusReader<KeyingEventToneChannel>>>>,
    keying_event_tone_channel_transform: Option<TransformBus<KeyingEvent, KeyingEventToneChannel>>,
    keyer_diag: Option<Arc<Mutex<dyn BusInput<KeyingEvent>>>>,
    keyer_diag_keying_event_rx: Option<Arc<Mutex<BusReader<KeyingEvent>>>>,
    source_encoder: Option<Arc<Mutex<dyn BusInput<KeyingEvent>>>>,
    source_encoder_keying_event_rx: Option<Arc<Mutex<BusReader<KeyingEvent>>>>,
}

impl Application {
    pub fn set_mode(&mut self, mode: Mode) {
        info!("Setting mode to {}", mode);
        self.mode = Some(mode);
        // TODO far more to do here, set up wiring for each Mode
        // There's always a KeyingEvent bus, and a receiver of it for the tone generator.
        let mut keying_event_bus = Bus::new(16);
        let tone_generator_keying_event_rx = keying_event_bus.add_rx();
        self.keying_event_bus = Some(Arc::new(Mutex::new(keying_event_bus)));

        { // Limit scope of temporaries..
            let mut transform_bus = Bus::new(16);
            let transform_bus_rx = transform_bus.add_rx();
            let keying_event_tone_channel_bus = Arc::new(Mutex::new(transform_bus));
            self.keying_event_tone_channel_bus = Some(keying_event_tone_channel_bus.clone());
            // SourceEncoderDiag needs a clone of this for playback
            self.keying_event_tone_channel_transform = Some(TransformBus::new(tone_generator_keying_event_rx,
                                                  keying_event_tone_channel_bus.clone(), add_sidetone_channel_to_keying_event,
                                                  self.terminate_flag.clone()));
            self.keying_event_tone_channel_rx = Some(Arc::new(Mutex::new(transform_bus_rx)));
        }

        match mode {
            Mode::KeyerDiag => {
                self.keyer_diag_keying_event_rx = Some(Arc::new(Mutex::new(self.keying_event_bus.as_ref().unwrap().lock().unwrap().add_rx())));
            }
            Mode::SourceEncoderDiag => {
                self.source_encoder_keying_event_rx = Some(Arc::new(Mutex::new(self.keying_event_bus.as_ref().unwrap().lock().unwrap().add_rx())));
            }
            _ => {
            }
        }
    }

    pub fn get_mode(&self) -> Option<Mode> {
        return self.mode.clone();
    }

    pub fn set_keyer(&mut self, keyer: Arc<Mutex<dyn BusOutput<KeyingEvent>>>) {
        info!("Starting to set keyer");
        match &self.keying_event_bus {
            None => {
                panic!("Cannot set a keyer with no keying_event_bus");
            }
            Some(keying_event_bus) => {
                info!("Setting keyer");
                self.keyer = Some(keyer.clone());
                let bus = keying_event_bus.clone();
                keyer.lock().as_mut().unwrap().set_output_tx(bus);
            }
        }
    }

    pub fn got_keyer(&self) -> bool {
        self.keyer.is_some()
    }

    pub fn clear_keyer(&mut self) {
        match &self.keyer {
            None => {}
            Some(keyer) => {
                info!("Clearing keyer");
                keyer.lock().unwrap().clear_output_tx();
            }
        }
        self.keyer = None;
    }

    pub fn set_tone_generator(&mut self, tone_generator: Arc<Mutex<dyn BusInput<KeyingEventToneChannel>>>) {
        info!("Starting to set tone generator");
        self.clear_tone_generator();
        match &self.keying_event_tone_channel_rx {
            None => {
                panic!("Cannot set a tone generator with no keying_event_tone_channel_rx");
            }
            Some(keying_event_tone_channel_rx) => {
                info!("Setting tone generator");
                self.tone_generator = Some(tone_generator.clone());
                let bus_reader = keying_event_tone_channel_rx.clone();
                tone_generator.lock().as_mut().unwrap().set_input_rx(bus_reader);
            }
        }
    }

    pub fn clear_tone_generator(&mut self) {
        match &self.tone_generator {
            None => {}
            Some(tone_generator) => {
                info!("Clearing tone generator");
                tone_generator.lock().unwrap().clear_input_rx();
            }
        }
        self.tone_generator = None;
    }

    pub fn got_tone_generator(&self) -> bool {
        self.tone_generator.is_some()
    }

    pub fn got_tone_generator_rx(&self) -> bool {
        self.keying_event_tone_channel_rx.is_some()
    }



    pub fn set_keyer_diag(&mut self, keyer_diag: Arc<Mutex<dyn BusInput<KeyingEvent>>>) {
        info!("Starting to set keyer diag");
        match &self.keyer_diag_keying_event_rx {
            None => {
                panic!("Cannot set a keyer_diag with no keyer_diag_keying_event_rx");
            }
            Some(keying_event_bus) => {
                info!("Setting keyer diag");
                self.keyer_diag = Some(keyer_diag.clone());
                let bus_reader = keying_event_bus.clone();
                keyer_diag.lock().as_mut().unwrap().set_input_rx(bus_reader);
            }
        }

    }

    pub fn clear_keyer_diag(&mut self) {
        match &self.keyer_diag {
            None => {}
            Some(keyer_diag) => {
                info!("Clearing keyer diag");
                keyer_diag.lock().unwrap().clear_input_rx();
            }
        }
        self.keyer_diag = None;
    }

    pub fn got_keyer_diag(&self) -> bool {
        self.keyer_diag.is_some()
    }

    pub fn got_keyer_diag_rx(&self) -> bool {
        self.keyer_diag_keying_event_rx.is_some()
    }


    pub fn set_source_encoder(&mut self, source_encoder: Arc<Mutex<dyn BusInput<KeyingEvent>>>) {
        info!("Starting to set source encoder");
        match &self.source_encoder_keying_event_rx {
            None => {
                panic!("Cannot set a source_encoder with no source_encoder_keying_event_rx");
            }
            Some(keying_event_bus) => {
                info!("Setting source encoder");
                self.source_encoder = Some(source_encoder.clone());
                let bus_reader = keying_event_bus.clone();
                source_encoder.lock().as_mut().unwrap().set_input_rx(bus_reader);
            }
        }
    }

    pub fn clear_source_encoder(&mut self) {
        // TODO unwire down
    }

    pub fn got_source_encoder(&self) -> bool {
        self.source_encoder.is_some()
    }

    pub fn got_source_encoder_rx(&self) -> bool {
        self.source_encoder_keying_event_rx.is_some()
    }


    // PortAudio functions...
    pub fn open_output_audio_device(&self, out_dev_str: &str) -> Result<OutputStreamSettings<f32>, Box<dyn Error>> {
        open_output_audio_device(&self.pa, out_dev_str)
    }

    pub fn open_input_audio_device(&self, in_dev_str: &str) -> Result<InputStreamSettings<f32>, Box<dyn Error>> {
        open_input_audio_device(&self.pa, in_dev_str)
    }

    pub fn pa_ref(&self) -> &PortAudio {
        self.pa.borrow()
    }
}

impl Application {
    pub fn new(terminate_flag: Arc<AtomicBool>,
               scheduled_thread_pool: Arc<ScheduledThreadPool>,
               pa: PortAudio,
    ) -> Self {
        debug!("Constructing Application");

        Self {
            terminate_flag,
            _scheduled_thread_pool: scheduled_thread_pool,
            pa,
            mode: None,

            keyer: None,
            keying_event_bus: None,
            tone_generator: None,
            keying_event_tone_channel_bus: None,
            keying_event_tone_channel_rx: None,
            keying_event_tone_channel_transform: None,
            keyer_diag: None,
            keyer_diag_keying_event_rx: None,
            source_encoder: None,
            source_encoder_keying_event_rx: None,
        }
    }

    // Setting the terminate AtomicBool will allow the thread to stop on its own.
    pub fn terminate(&mut self) {
        info!("Terminating Application");
        self.terminate_flag.store(true, core::sync::atomic::Ordering::SeqCst);
        info!("Terminated Application");
    }

    // Has the Application been terminated
    pub fn terminated(&self) -> bool {
        debug!("Is Application terminated?");
        let ret = self.terminate_flag.load(core::sync::atomic::Ordering::SeqCst);
        debug!("Termination state is {}", ret);
        ret
    }

    // Obtain a clone of the global terminate flag.
    pub fn terminate_flag(&mut self) -> Arc<AtomicBool> {
        self.terminate_flag.clone()
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