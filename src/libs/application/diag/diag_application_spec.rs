/*
 * These tests are manually invoked and validate that the application provides a suitably
 * performant backbone for the rest of the system.
 * They assume the existence of a valid configuration file as created by main.
 */
#[cfg(test)]
mod diag_application_spec {
    use std::env;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};

    use log::{debug, info};
    use portaudio as pa;
    use portaudio::PortAudio;
    use rstest::*;
    use syncbox::ScheduledThreadPool;

    use crate::libs::application::application::{Application, Mode};
    use crate::libs::config_dir::config_dir;
    use crate::libs::config_file::config_file::ConfigurationStore;
    use crate::libs::keyer_io::arduino_keyer_io::ArduinoKeyer;
    use crate::libs::keyer_io::keyer_io::{Keyer, KeyerSpeed};
    use crate::libs::serial_io::serial_io::DefaultSerialIO;
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct DiagApplicationFixture {
        config: ConfigurationStore,
        terminate: Arc<AtomicBool>,
        _scheduled_thread_pool: Arc<ScheduledThreadPool>,
        application: Application,
        _pa: PortAudio,
    }

    #[fixture]
    fn fixture() -> DiagApplicationFixture {
        let home_dir = dirs::home_dir();
        let config_path = config_dir::configuration_directory(home_dir).unwrap();
        debug!("Configuration path is [{:?}]", config_path);
        let config = ConfigurationStore::new(config_path).unwrap();
        debug!("Configuration file is [{:?}]", config.get_config_file_path());

        let terminate = Arc::new(AtomicBool::new(false));
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

        let application = Application::new(terminate.clone(), scheduled_thread_pool.clone());

        let pa = pa::PortAudio::new();
        if pa.is_err() {
            panic!("Portaudio setup failure: {:?}", pa);
        }

        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        DiagApplicationFixture {
            config,
            terminate,
            _scheduled_thread_pool: scheduled_thread_pool,
            application,
            _pa: pa.unwrap(),
        }
    }

    fn set_keyer(config: &mut ConfigurationStore, application: &mut Application) {
        info!("Setting up keyer");
        let port_string = config.get_port();
        let port = port_string.as_str();

        info!("Initialising serial port at {}", port);
        let serial_io = DefaultSerialIO::new(port.to_string());
        match serial_io {
            Ok(_) => {}
            Err(err) => {
                panic!("DefaultSerialIO setup failure: {}", err);
            }
        }
        let mut keyer = ArduinoKeyer::new(Box::new(serial_io.unwrap()), application.terminate_flag());
        let keyer_speed: KeyerSpeed = config.get_wpm() as KeyerSpeed;
        match keyer.set_speed(keyer_speed) {
            Ok(_) => {}
            Err(err) => {
                panic!("Can't set keyer speed to {}: {}", keyer_speed, err);
            }
        }

        application.set_keyer(Arc::new(Mutex::new(keyer)));
    }

    impl Drop for DiagApplicationFixture {
        fn drop(&mut self) {
            debug!("ApplicationFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("ApplicationFixture ...set terminate flag");
        }
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn mode_keyer_diag(mut fixture: DiagApplicationFixture) {
        debug!("start mode_keyer_diag");
        fixture.application.set_mode(Mode::KeyerDiag);
        set_keyer(&mut fixture.config, &mut fixture.application);

        // TODO add the tone generator and extracted keyer diag objects.
        debug!("end mode_keyer_diag");
    }
}
