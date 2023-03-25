use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use bus::{Bus, BusReader};
use log::info;
use digimorse::libs::application::application::{Application, ApplicationMode, BusInput, BusOutput};
use digimorse::libs::config_dir::config_dir;
use digimorse::libs::config_file::config_file::ConfigurationStore;
use digimorse::libs::util::logging::initialise_logging;

use portaudio as pa;
use digimorse::libs::audio::tone_generator::ToneGenerator;
use digimorse::libs::gui::gui::Gui;
use digimorse::libs::keyer_io::keyer_io::{KeyerSpeed, KeyingEvent};
use digimorse::libs::source_codec::source_encoding::SourceEncoding;

struct NullSourceEncoder {
    bus_reader: Arc<Mutex<Option<Arc<Mutex<BusReader<KeyingEvent>>>>>>,
}
impl NullSourceEncoder {
    fn new(terminate: Arc<AtomicBool>) -> Self {
        let input_rx_holder = Arc::new(Mutex::new(None));
        let thread_input_rx_holder = input_rx_holder.clone();
        thread::spawn(move || {
            loop {
                if terminate.load(Ordering::SeqCst) {
                    break;
                }
                let mut need_sleep = false;
                match thread_input_rx_holder.lock().unwrap().as_deref() {
                    None => {
                        // Input channel hasn't been set yet; sleep, after releasing lock
                        need_sleep = true;
                    }
                    Some(input_rx) => {
                        match (input_rx as &Mutex<BusReader<KeyingEvent>>).lock().unwrap().recv_timeout(Duration::from_millis(100)) {
                            Ok(keying_event) => {
                                info!("Throwing away {}", keying_event);
                            }
                            Err(_) => {
                                // Don't log, it's just noise - timeout gives opportunity to go round loop and
                                // check for terminate.
                            }
                        }
                    }
                }
                if need_sleep {
                    thread::sleep(Duration::from_millis(100));
                }
            }
            info!("Null source encoder terminated");
        });
        Self {
            bus_reader: input_rx_holder
        }
    }
}

impl BusInput<KeyingEvent> for NullSourceEncoder {
    fn clear_input_rx(&mut self) {
        match self.bus_reader.lock() {
            Ok(mut locked) => { *locked = None; }
            Err(_) => {}
        }
    }

    fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<KeyingEvent>>>) {
        match self.bus_reader.lock() {
            Ok(mut locked) => { *locked = Some(input_rx); }
            Err(_) => {}
        }
    }
}

impl BusOutput<SourceEncoding> for NullSourceEncoder {
    fn clear_output_tx(&mut self) {
    }

    fn set_output_tx(&mut self, _output_tx: Arc<Mutex<Bus<SourceEncoding>>>) {
    }
}


fn main() {
    initialise_logging();
    info!("GUI test harness");
    let terminate = Arc::new(AtomicBool::new(false));
    let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

    let home_dir = dirs::home_dir();
    let config_path = config_dir::configuration_directory(home_dir).unwrap();
    let config = ConfigurationStore::new(config_path).unwrap();
    info!("Initialising PortAudio");
    let pa = pa::PortAudio::new().unwrap();
    let mut application = Application::new(terminate.clone(), scheduled_thread_pool.clone(), pa);
    application.set_ctrlc_handler();
    application.set_mode(ApplicationMode::Full);
    application.set_keyer_speed(config.get_wpm() as KeyerSpeed);

    // Attach the tone generator to hear the sidetone...
    info!("Initialising audio output callback...");
    let out_dev_string = config.get_audio_out_device();
    let out_dev_str = out_dev_string.as_str();
    let output_settings = application.open_output_audio_device(out_dev_str).expect("Could not initialise audio output");
    let mut tone_generator = ToneGenerator::new(config.get_sidetone_frequency(),
                                                application.terminate_flag());
    tone_generator.start_callback(application.pa_ref(), output_settings).expect("Could not initialise tone generator callback");
    let application_tone_generator = Arc::new(Mutex::new(tone_generator));
    // let playback_arc_mutex_tone_generator = application_tone_generator.clone();
    application.set_tone_generator(application_tone_generator);

    // The keying will be sent to on the source encoder input bus, and this has to be thrown away or
    // else the bus will fill and lock up the program.
    let null_source_encoder_terminate = terminate.clone();
    application.set_source_encoder(Arc::new(Mutex::new(NullSourceEncoder::new(null_source_encoder_terminate))));

    info!("Initialising GUI");
    let gui_config = Arc::new(Mutex::new(config));
    let gui_application = Arc::new(Mutex::new(application));
    let terminate_application = gui_application.clone();
    let mut gui = Gui::new(gui_config, gui_application);
    gui.message_loop();
    info!("End of GUI harness; terminating...");
    terminate_application.lock().unwrap().terminate();
    thread::sleep(Duration::from_secs(1));
    info!("Exiting");
}