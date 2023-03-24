use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;
use log::info;
use digimorse::libs::application::application::{Application, ApplicationMode};
use digimorse::libs::config_dir::config_dir;
use digimorse::libs::config_file::config_file::ConfigurationStore;
use digimorse::libs::util::logging::initialise_logging;

use portaudio as pa;
use digimorse::libs::gui::gui::Gui;
use digimorse::libs::keyer_io::keyer_io::KeyerSpeed;

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