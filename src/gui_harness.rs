use fltk::app::*;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use bus::{Bus, BusReader};
use digimorse::libs::gui::gui_facades::GUIOutput;
use log::info;
use digimorse::libs::application::application::{Application, ApplicationMode, BusInput, BusOutput};
use digimorse::libs::config_dir::config_dir;
use digimorse::libs::config_file::config_file::ConfigurationStore;
use digimorse::libs::util::logging::initialise_logging;

use portaudio as pa;
use digimorse::libs::audio::tone_generator::ToneGenerator;
use digimorse::libs::gui::gui::{Gui, WIDGET_PADDING};
use digimorse::libs::gui::gui_driver::GuiDriver;
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
                        if let Ok(keying_event) = (input_rx as &Mutex<BusReader<KeyingEvent>>).lock().unwrap().recv_timeout(Duration::from_millis(100)) {
                            info!("Throwing away {}", keying_event);
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
        if let Ok(mut locked) = self.bus_reader.lock() {
            *locked = None;
        }
    }

    fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<KeyingEvent>>>) {
        if let Ok(mut locked) = self.bus_reader.lock() {
            *locked = Some(input_rx);
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
    let gui_terminate = terminate.clone();
    let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

    let home_dir = dirs::home_dir();
    let config_path = config_dir::configuration_directory(home_dir).unwrap();
    let config = ConfigurationStore::new(config_path).unwrap();
    info!("Initialising PortAudio");
    let pa = pa::PortAudio::new().unwrap();
    let application = Application::new(terminate.clone(), scheduled_thread_pool, pa);
    let arc_mutex_application = Arc::new(Mutex::new(application));
    let application_gui_output: Arc<Mutex<dyn GUIOutput>> = arc_mutex_application.clone() as Arc<Mutex<dyn GUIOutput>>;
    arc_mutex_application.lock().unwrap().set_ctrlc_handler();
    arc_mutex_application.lock().unwrap().set_mode(ApplicationMode::Full);
    arc_mutex_application.lock().unwrap().set_keyer_speed(config.get_wpm() as KeyerSpeed);


    // Attach the tone generator to hear the sidetone...
    info!("Initialising audio output callback...");
    let out_dev_string = config.get_audio_out_device();
    let out_dev_str = out_dev_string.as_str();
    let output_settings = arc_mutex_application.lock().unwrap().open_output_audio_device(out_dev_str).expect("Could not initialise audio output");
    let mut tone_generator = ToneGenerator::new(config.get_sidetone_frequency(),
                                                arc_mutex_application.lock().unwrap().terminate_flag());
    tone_generator.start_callback(arc_mutex_application.lock().unwrap().pa_ref(), output_settings).expect("Could not initialise tone generator callback");
    let application_tone_generator = Arc::new(Mutex::new(tone_generator));
    // let playback_arc_mutex_tone_generator = application_tone_generator.clone();
    arc_mutex_application.lock().unwrap().set_tone_generator(application_tone_generator);

    // The keying will be sent to on the source encoder input bus, and this has to be thrown away or
    // else the bus will fill and lock up the program.
    //let null_source_encoder_terminate = terminate.clone();
    arc_mutex_application.lock().unwrap().set_source_encoder(Arc::new(Mutex::new(NullSourceEncoder::new(terminate))));

    info!("Initialising GUI");
    let gui_config = Arc::new(Mutex::new(config));
    let terminate_application = arc_mutex_application.clone();
    let app = App::default().with_scheme(Scheme::Gtk);
    let gui = Gui::new(gui_config, application_gui_output, gui_terminate);
    let gui_width = gui.main_window_dimensions().0;
    let gui_input = gui.gui_input_sender();
    let arc_mutex_gui = Arc::new(Mutex::new(gui));
    // The gui_input would be passed around to other parts of the subsystem.
    GuiDriver::new(gui_input, gui_width + WIDGET_PADDING);
    info!("Start of app wait loop");
    while app.wait() {
        arc_mutex_gui.lock().unwrap().message_handle();
    }
    info!("End of app wait loop");
    info!("End of GUI harness; terminating...");
    terminate_application.lock().unwrap().terminate();
    thread::sleep(Duration::from_secs(1));
    info!("Exiting");
}
