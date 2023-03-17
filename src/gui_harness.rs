use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;
use log::info;
use digimorse::libs::application::application::{Application, ApplicationMode};
use digimorse::libs::config_dir::config_dir;
use digimorse::libs::config_file::config_file::ConfigurationStore;
use digimorse::libs::gui::gui;
use digimorse::libs::util::logging::initialise_logging;

use portaudio as pa;

fn main() {
    initialise_logging();
    info!("GUI test harness");
    let terminate = Arc::new(AtomicBool::new(false));
    let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

    let home_dir = dirs::home_dir();
    let config_path = config_dir::configuration_directory(home_dir).unwrap();
    let mut config = ConfigurationStore::new(config_path).unwrap();
    info!("Initialising PortAudio");
    let pa = pa::PortAudio::new().unwrap();
    let mut application = Application::new(terminate.clone(), scheduled_thread_pool.clone(), pa);
    application.set_ctrlc_handler();
    application.set_mode(ApplicationMode::Full);

    info!("Initialising GUI");
    gui::initialise(&mut config, &mut application);
    info!("End of test harness");
    application.terminate();
    thread::sleep(Duration::from_secs(5));
    info!("Exiting");
}