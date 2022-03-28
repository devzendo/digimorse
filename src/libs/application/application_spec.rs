extern crate hamcrest2;

#[cfg(test)]
mod application_spec {
    use std::env;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use bus::{Bus, BusReader};
    use hamcrest2::prelude::*;
    use log::{debug, info, warn};
    use rstest::*;
    use syncbox::ScheduledThreadPool;

    use crate::libs::application::application::{Application, BusInput, BusOutput, Mode};
    use crate::libs::keyer_io::keyer_io::KeyingEvent;
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct ApplicationFixture {
        terminate: Arc<AtomicBool>,
        _scheduled_thread_pool: Arc<ScheduledThreadPool>,
        application: Application,
    }

    #[fixture]
    fn fixture() -> ApplicationFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

        let application = Application::new(terminate.clone(), scheduled_thread_pool.clone());

        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        ApplicationFixture {
            terminate,
            _scheduled_thread_pool: scheduled_thread_pool,
            application,
        }
    }

    impl Drop for ApplicationFixture {
        fn drop(&mut self) {
            debug!("ApplicationFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("ApplicationFixture ...set terminate flag");
        }
    }


    struct FakeKeyer {
        keying: Vec<KeyingEvent>,
        bus: Option<Arc<Mutex<Bus<KeyingEvent>>>>,
    }

    impl BusOutput<KeyingEvent> for FakeKeyer {
        fn clear_output_tx(&mut self) {
            self.bus = None;
        }

        fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<KeyingEvent>>>) {
            self.bus = Some(output_tx);
        }
    }

    impl FakeKeyer {
        fn new(keying: Vec<KeyingEvent>) -> Self {
            Self {
                keying,
                bus: None
            }
        }

        fn got_output_tx(&self) -> bool {
            self.bus.is_some()
        }

        fn start(&mut self) {
            match self.bus.clone() {
                None => {
                    warn!("No bus set in FakeKeyer");
                }
                Some(bus) => {
                    info!("Sending keying from FakeKeyer");
                    let mut guard = bus.lock().unwrap();
                    for v in &self.keying {
                        guard.broadcast(*v);
                    }
                    info!("Sent keying from FakeKeyer");
                }
            }
        }
    }

    struct StubBusReader {
        keying: Vec<KeyingEvent>,
        bus_reader: Option<Arc<Mutex<BusReader<KeyingEvent>>>>,
    }

    impl BusInput<KeyingEvent> for StubBusReader {
        fn clear_input_rx(&mut self) {
            self.bus_reader = None;
        }

        fn set_input_rx(&mut self, input_tx: Arc<Mutex<BusReader<KeyingEvent>>>) {
            self.bus_reader = Some(input_tx);
        }
    }

    impl StubBusReader {
        fn new() -> Self {
            Self {
                keying: vec![],
                bus_reader: None
            }
        }

        fn got_input_rx(&self) -> bool {
            self.bus_reader.is_some()
        }

        fn read(&mut self) -> Vec<KeyingEvent> {
            match &self.bus_reader {
                None => {
                    panic!("No bus reader set in StubBusReader");
                }
                Some(bus_reader) => {
                    loop {
                        match bus_reader.clone().lock().unwrap().recv_timeout(Duration::from_millis(500)) {
                            Ok(ke) => {
                                self.keying.push(ke);
                            }
                            Err(_) => {
                                info!("StubBusReader timed out on read");
                                break;
                            }
                        }
                    }
                }
            }
            info!("Out of StubBusReader read");
            self.keying.clone()
        }
    }


    #[rstest]
    pub fn termination(mut fixture: ApplicationFixture) {
        assert_eq!(fixture.application.terminated(), false);
        test_util::wait_5_ms();
        fixture.application.terminate();
        test_util::wait_5_ms();
        assert_eq!(fixture.application.terminated(), true);
        assert_eq!(fixture.terminate.load(Ordering::SeqCst), true);
    }

    #[rstest]
    pub fn initial_mode(fixture: ApplicationFixture) {
        assert_that!(fixture.application.get_mode(), none());
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), false);
        assert_eq!(fixture.application.got_tone_generator(), false);
        assert_eq!(fixture.application.got_tone_generator_rx(), false);
        assert_eq!(fixture.application.got_source_encoder(), false);
        assert_eq!(fixture.application.got_source_encoder_rx(), false);
    }

    #[rstest]
    pub fn mode_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(Mode::KeyerDiag);
        assert_that!(fixture.application.get_mode(), has(Mode::KeyerDiag));
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
        assert_eq!(fixture.application.got_tone_generator(), false);
        assert_eq!(fixture.application.got_tone_generator_rx(), true);
        assert_eq!(fixture.application.got_source_encoder(), false);
        assert_eq!(fixture.application.got_source_encoder_rx(), false);
    }

    #[rstest]
    pub fn set_clear_keyer(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(Mode::KeyerDiag);
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
        let keyer = Arc::new(Mutex::new(FakeKeyer::new(vec![])));
        fixture.application.set_keyer(keyer);
        assert_eq!(fixture.application.got_keyer(), true);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
        fixture.application.clear_keyer();
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
    }

    #[rstest]
    pub fn set_clear_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(Mode::KeyerDiag);
        assert_eq!(fixture.application.got_tone_generator(), false);
        assert_eq!(fixture.application.got_tone_generator_rx(), true);
        let tone_generator = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_tone_generator(tone_generator);
        assert_eq!(fixture.application.got_tone_generator(), true);
        assert_eq!(fixture.application.got_tone_generator_rx(), true);
        fixture.application.clear_tone_generator();
        assert_eq!(fixture.application.got_tone_generator(), false);
        assert_eq!(fixture.application.got_tone_generator_rx(), true);
    }

    #[rstest]
    pub fn set_clear_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(Mode::KeyerDiag);
        assert_eq!(fixture.application.got_keyer_diag(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
        let keyer_diag = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_keyer_diag(keyer_diag);
        assert_eq!(fixture.application.got_keyer_diag(), true);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
        fixture.application.clear_keyer_diag();
        assert_eq!(fixture.application.got_keyer_diag(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
    }

    #[rstest]
    // No need for the FakeKeyer and StubBusReader to have their own threads, so long as no more
    // than 16 elements are placed onto the bus.
    pub fn keyer_diag_bus_wiring(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(Mode::KeyerDiag);

        let sent_keying = vec![KeyingEvent::Start(), KeyingEvent::End()];
        let keyer = Arc::new(Mutex::new(FakeKeyer::new(sent_keying.clone())));
        assert_that!(keyer.lock().unwrap().got_output_tx(), false);
        let application_keyer = keyer.clone();
        fixture.application.set_keyer(application_keyer);
        assert_that!(keyer.lock().unwrap().got_output_tx(), true);

        let tone_generator = Arc::new(Mutex::new(StubBusReader::new()));
        assert_that!(tone_generator.lock().unwrap().got_input_rx(), false);
        let application_tone_generator = tone_generator.clone();
        fixture.application.set_tone_generator(application_tone_generator);
        assert_that!(tone_generator.lock().unwrap().got_input_rx(), true);

        let keyer_diag = Arc::new(Mutex::new(StubBusReader::new()));
        assert_that!(keyer_diag.lock().unwrap().got_input_rx(), false);
        let application_keyer_diag = keyer_diag.clone();
        fixture.application.set_keyer_diag(application_keyer_diag);
        assert_that!(keyer_diag.lock().unwrap().got_input_rx(), true);


        keyer.lock().unwrap().start();
        info!("Test sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Test out of sleep");


        let tone_generator_received_keying = tone_generator.lock().unwrap().read();
        let keyer_diag_received_keying = keyer_diag.lock().unwrap().read();

        assert_eq!(tone_generator_received_keying, sent_keying.clone());
        assert_eq!(keyer_diag_received_keying, sent_keying.clone());
    }

    #[rstest]
    pub fn keyer_diag_bus_unwiring(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(Mode::KeyerDiag);

        let keyer = Arc::new(Mutex::new(FakeKeyer::new(vec![])));
        let application_keyer = keyer.clone();
        fixture.application.set_keyer(application_keyer);

        let tone_generator = Arc::new(Mutex::new(StubBusReader::new()));
        let application_tone_generator = tone_generator.clone();
        fixture.application.set_tone_generator(application_tone_generator);

        let keyer_diag = Arc::new(Mutex::new(StubBusReader::new()));
        let application_keyer_diag = keyer_diag.clone();
        fixture.application.set_keyer_diag(application_keyer_diag);


        keyer.lock().unwrap().start();
        info!("Test sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Test out of sleep");

        fixture.application.clear_keyer();
        assert_eq!(keyer.lock().unwrap().got_output_tx(), false);
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);

        assert_eq!(fixture.application.got_tone_generator(), true);
        assert_eq!(fixture.application.got_tone_generator_rx(), true);

        assert_eq!(fixture.application.got_keyer_diag(), true);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);

        fixture.application.clear_tone_generator();
        assert_eq!(tone_generator.lock().unwrap().got_input_rx(), false);

        assert_eq!(fixture.application.got_tone_generator(), false);
        assert_eq!(fixture.application.got_tone_generator_rx(), true);

        assert_eq!(fixture.application.got_keyer_diag(), true);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);

        fixture.application.clear_keyer_diag();
        assert_eq!(keyer_diag.lock().unwrap().got_input_rx(), false);

        assert_eq!(fixture.application.got_keyer_diag(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
    }

}
