extern crate portaudio;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
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
use crate::libs::channel_codec::channel_encoder::ChannelEncoder;
use crate::libs::channel_codec::channel_encoding::ChannelEncoding;
use crate::libs::source_codec::source_encoder::SourceEncoderTrait;
use crate::libs::source_codec::source_encoding::SourceEncoding;

arg_enum! {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ApplicationMode {
        Full,
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
    fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<T>>>);
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
    scheduled_thread_pool: Arc<ScheduledThreadPool>,
    pa: PortAudio,
    mode: Option<ApplicationMode>,

    keyer: Option<Arc<Mutex<dyn BusOutput<KeyingEvent>>>>,
    keying_event_bus: Option<Arc<Mutex<Bus<KeyingEvent>>>>,
    tone_generator: Option<Arc<Mutex<dyn BusInput<KeyingEventToneChannel>>>>,
    keying_event_tone_channel_bus: Option<Arc<Mutex<Bus<KeyingEventToneChannel>>>>,
    keying_event_tone_channel_rx: Option<Arc<Mutex<BusReader<KeyingEventToneChannel>>>>,
    keying_event_tone_channel_transform: Option<TransformBus<KeyingEvent, KeyingEventToneChannel>>,
    keyer_diag: Option<Arc<Mutex<dyn BusInput<KeyingEvent>>>>,
    keyer_diag_keying_event_rx: Option<Arc<Mutex<BusReader<KeyingEvent>>>>,
    source_encoder: Option<Arc<Mutex<dyn SourceEncoderTrait>>>,
    source_encoding_bus: Option<Arc<Mutex<Bus<SourceEncoding>>>>,
    source_encoder_keying_event_rx: Option<Arc<Mutex<BusReader<KeyingEvent>>>>,
    source_encoder_diag: Option<Arc<Mutex<dyn BusInput<SourceEncoding>>>>,
    source_encoder_diag_source_encoding_rx: Option<Arc<Mutex<BusReader<SourceEncoding>>>>,
    channel_encoder: Option<Arc<Mutex<ChannelEncoder>>>,
    channel_encoding_bus: Option<Arc<Mutex<Bus<ChannelEncoding>>>>,
    channel_encoder_source_encoding_rx: Option<Arc<Mutex<BusReader<SourceEncoding>>>>,
    playback: Option<Arc<Mutex<dyn BusOutput<KeyingEventToneChannel>>>>,
}

impl Application {
    pub fn set_mode(&mut self, mode: ApplicationMode) {
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
            let mut bus = TransformBus::new(
                                        add_sidetone_channel_to_keying_event,
                                        self.terminate_flag.clone());
            bus.set_input_rx(Arc::new(Mutex::new(tone_generator_keying_event_rx)));
            bus.set_output_tx(keying_event_tone_channel_bus.clone());
            self.keying_event_tone_channel_transform = Some(bus);
            self.keying_event_tone_channel_rx = Some(Arc::new(Mutex::new(transform_bus_rx)));
        }

        if mode == ApplicationMode::SourceEncoderDiag || mode == ApplicationMode::Full {
            let source_encoding_bus = Bus::new(16);
            self.source_encoding_bus = Some(Arc::new(Mutex::new(source_encoding_bus)));
        }

        if mode == ApplicationMode::Full {
            let channel_encoding_bus = Bus::new(16);
            self.channel_encoding_bus = Some(Arc::new(Mutex::new(channel_encoding_bus)));
        }

        match mode {
            ApplicationMode::KeyerDiag => {
                self.keyer_diag_keying_event_rx = Some(Arc::new(Mutex::new(self.keying_event_bus.as_ref().unwrap().lock().unwrap().add_rx())));
            }
            ApplicationMode::SourceEncoderDiag => {
                self.source_encoder_keying_event_rx = Some(Arc::new(Mutex::new(self.keying_event_bus.as_ref().unwrap().lock().unwrap().add_rx())));
                self.source_encoder_diag_source_encoding_rx = Some(Arc::new(Mutex::new(self.source_encoding_bus.as_ref().unwrap().lock().unwrap().add_rx())));
            }
            ApplicationMode::Full => {
                self.source_encoder_keying_event_rx = Some(Arc::new(Mutex::new(self.keying_event_bus.as_ref().unwrap().lock().unwrap().add_rx())));
                self.channel_encoder_source_encoding_rx = Some(Arc::new(Mutex::new(self.source_encoding_bus.as_ref().unwrap().lock().unwrap().add_rx())));
            }
        }
    }

    pub fn get_mode(&self) -> Option<ApplicationMode> {
        return self.mode.clone();
    }

    pub fn set_keyer(&mut self, keyer: Arc<Mutex<dyn BusOutput<KeyingEvent>>>) {
        if self.mode.is_none() {
            panic!("Can't set keyer in mode {:?}", self.mode);
        }
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
        if self.mode.is_none() {
            panic!("Can't clear keyer in mode {:?}", self.mode);
        }
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
        if self.mode.is_none() {
            panic!("Can't set tone_generator in mode {:?}", self.mode);
        }
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
        if self.mode.is_none() {
            panic!("Can't clear tone_generator in mode {:?}", self.mode);
        }
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
        if self.mode.is_none() || self.mode.unwrap() != ApplicationMode::KeyerDiag {
            panic!("Can't set keyer_diag in mode {:?}", self.mode);
        }
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
        if self.mode.is_none() || self.mode.unwrap() != ApplicationMode::KeyerDiag {
            panic!("Can't clear keyer_diag in mode {:?}", self.mode);
        }
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


    pub fn set_source_encoder(&mut self, source_encoder: Arc<Mutex<dyn SourceEncoderTrait>>) {
        if self.mode.is_none() || self.mode.unwrap() == ApplicationMode::KeyerDiag {
            panic!("Can't set source_encoder in mode {:?}", self.mode);
        }
        info!("Starting to set source encoder");
        match &self.source_encoder_keying_event_rx {
            None => {
                panic!("Cannot set a source_encoder with no source_encoder_keying_event_rx");
            }
            Some(keying_event_bus) => {
                info!("Setting source encoder");
                self.source_encoder = Some(source_encoder.clone());
                info!("Setting source encoder input");
                let bus_reader = keying_event_bus.clone();
                source_encoder.lock().as_mut().unwrap().set_input_rx(bus_reader);
            }
        }
        match &self.source_encoding_bus {
            None => {
                panic!("Cannot set a source_encoder's output with no source_encoding_bus");
            }
            Some(source_encoding_bus) => {
                info!("Setting source encoder output");
                source_encoder.lock().as_mut().unwrap().set_output_tx(source_encoding_bus.clone());
            }
        }
    }

    pub fn clear_source_encoder(&mut self) {
        if self.mode.is_none() ||
            (self.mode.unwrap() != ApplicationMode::SourceEncoderDiag &&
                self.mode.unwrap() != ApplicationMode::Full) {
            panic!("Can't clear source_encoder in mode {:?}", self.mode);
        }
        match &self.source_encoder {
            None => {}
            Some(source_encoder) => {
                info!("Clearing source encooder");
                source_encoder.lock().unwrap().clear_input_rx();
                source_encoder.lock().unwrap().clear_output_tx();
            }
        }
        self.source_encoder = None;
    }

    pub fn got_source_encoder(&self) -> bool {
        self.source_encoder.is_some()
    }

    pub fn got_source_encoder_keying_event_rx(&self) -> bool {
        self.source_encoder_keying_event_rx.is_some()
    }


    pub fn got_source_encoder_diag_source_encoding_rx(&self) -> bool {
        self.source_encoder_diag_source_encoding_rx.is_some()
    }

    pub fn set_source_encoder_diag(&mut self, source_encoder_diag: Arc<Mutex<dyn BusInput<SourceEncoding>>>) {
        if self.mode.is_none() || self.mode.unwrap() != ApplicationMode::SourceEncoderDiag {
            panic!("Can't set source_encoder_diag in mode {:?}", self.mode);
        }
        info!("Starting to set source encoder diag");
        match &self.source_encoder_diag_source_encoding_rx {
            None => {
                panic!("Cannot set a source_encoder_diag with no source_encoder_diag_source_encoding_rx");
            }
            Some(source_encoding_bus) => {
                info!("Setting source encoder diag");
                self.source_encoder_diag = Some(source_encoder_diag.clone());
                let bus_reader = source_encoding_bus.clone();
                source_encoder_diag.lock().as_mut().unwrap().set_input_rx(bus_reader);
            }
        }

    }

    pub fn clear_source_encoder_diag(&mut self) {
        if self.mode.is_none() || self.mode.unwrap() != ApplicationMode::SourceEncoderDiag {
            panic!("Can't clear source_encoder_diag in mode {:?}", self.mode);
        }

        match &self.source_encoder_diag {
            None => {}
            Some(source_encoder_diag) => {
                info!("Clearing source encoder diag");
                source_encoder_diag.lock().unwrap().clear_input_rx();
            }
        }
        self.source_encoder_diag = None;
    }

    pub fn got_source_encoder_diag(&self) -> bool {
        self.source_encoder_diag.is_some()
    }

    pub fn got_source_encoder_diag_rx(&self) -> bool {
        self.source_encoder_diag_source_encoding_rx.is_some()
    }



    pub fn set_channel_encoder(&mut self, channel_encoder: Arc<Mutex<ChannelEncoder>>) {
        if self.mode.is_none() || self.mode.unwrap() != ApplicationMode::Full {
            panic!("Can't set channel_encoder in mode {:?}", self.mode);
        }
        info!("Starting to set channel encoder");
        match &self.channel_encoder_source_encoding_rx {
            None => {
                panic!("Cannot set a channel_encoder with no channel_encoder_source_encoding_rx");
            }
            Some(source_encoding_bus) => {
                info!("Setting channel encoder");
                self.channel_encoder = Some(channel_encoder.clone());
                info!("Setting channel encoder input");
                let bus_reader = source_encoding_bus.clone();
                channel_encoder.lock().as_mut().unwrap().set_input_rx(bus_reader);
            }
        }
        match &self.channel_encoding_bus {
            None => {
                panic!("Cannot set a channel_encoder's output with no channel_encoding_bus");
            }
            Some(channel_encoding_bus) => {
                info!("Setting channel encoder output");
                channel_encoder.lock().as_mut().unwrap().set_output_tx(channel_encoding_bus.clone());
            }
        }
    }

    pub fn clear_channel_encoder(&mut self) {
        if self.mode.is_none() ||
            self.mode.unwrap() != ApplicationMode::Full {
            panic!("Can't clear channel_encoder in mode {:?}", self.mode);
        }
        match &self.channel_encoder {
            None => {}
            Some(channel_encoder) => {
                info!("Clearing channel encoder");
                channel_encoder.lock().unwrap().clear_input_rx();
                channel_encoder.lock().unwrap().clear_output_tx();
            }
        }
        self.channel_encoder = None;
    }

    pub fn got_channel_encoder(&self) -> bool {
        self.channel_encoder.is_some()
    }

    pub fn got_channel_encoder_source_encoding_rx(&self) -> bool {
        self.channel_encoder_source_encoding_rx.is_some()
    }



    pub fn set_playback(&mut self, playback: Arc<Mutex<dyn BusOutput<KeyingEventToneChannel>>>) {
        if self.mode.is_none() || self.mode.unwrap() == ApplicationMode::KeyerDiag {
            panic!("Can't set playback in mode {:?}", self.mode);
        }
        info!("Starting to set playback");
        match &self.keying_event_tone_channel_bus {
            None => {
                panic!("Can't set playback's output bus because it doesn't exist");
            }
            Some(keying_event_tone_channel_bus) => {
                info!("Setting playback");
                self.playback = Some(playback.clone());
                playback.lock().unwrap().set_output_tx(keying_event_tone_channel_bus.clone());
            }
        }
    }

    pub fn clear_playback(&mut self) {
        if self.mode.is_none() || self.mode.unwrap() == ApplicationMode::KeyerDiag {
            panic!("Can't clear playback in mode {:?}", self.mode);
        }
        match &self.playback {
            None => {}
            Some(playback) => {
                info!("Clearing playback");
                playback.lock().unwrap().clear_output_tx();
            }
        }
        self.playback = None;
    }

    pub fn got_playback(&self) -> bool {
        self.playback.is_some()
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
            scheduled_thread_pool,
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
            source_encoding_bus: None,
            source_encoder_keying_event_rx: None,
            source_encoder_diag: None,
            source_encoder_diag_source_encoding_rx: None,
            channel_encoder: None,
            channel_encoding_bus: None,
            channel_encoder_source_encoding_rx: None,
            playback: None,
        }
    }

    // Initialise the Ctrl-C handler. Called once by the application.
    pub fn set_ctrlc_handler(&mut self) {
        debug!("Setting Ctrl-C handler");
        let ctrlc_arc_terminate = self.terminate_flag();
        ctrlc::set_handler(move || {
            info!("Setting terminate flag...");
            ctrlc_arc_terminate.store(true, Ordering::SeqCst);
            info!("... terminate flag set");
        }).expect("Error setting Ctrl-C handler");
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

    // Obtain a clone of the global scheduled thread pool.
    pub fn scheduled_thread_pool(&mut self) -> Arc<ScheduledThreadPool> {
        self.scheduled_thread_pool.clone()
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