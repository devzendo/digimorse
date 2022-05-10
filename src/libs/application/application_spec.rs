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
    use portaudio as pa;
    use rstest::*;
    use syncbox::ScheduledThreadPool;

    use crate::libs::application::application::{Application, BusInput, BusOutput, ApplicationMode};
    use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneChannel};
    use crate::libs::delayed_bus::delayed_bus::DelayedBus;
    use crate::libs::keyer_io::keyer_io::{KeyerSpeed, KeyingEvent};
    use crate::libs::source_codec::source_encoder::SourceEncoder;
    use crate::libs::source_codec::source_encoding::SOURCE_ENCODER_BLOCK_SIZE_IN_BITS;
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
        scheduled_thread_pool: Arc<ScheduledThreadPool>,
        application: Application,
    }

    #[fixture]
    fn fixture() -> ApplicationFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

        let pa = pa::PortAudio::new();
        if pa.is_err() {
            panic!("Portaudio setup failure: {:?}", pa);
        }

        let application = Application::new(terminate.clone(), scheduled_thread_pool.clone(), pa.unwrap());

        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        ApplicationFixture {
            terminate,
            scheduled_thread_pool: scheduled_thread_pool,
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

        fn start_sending(&mut self) {
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

    struct StubBusReader<T> {
        content: Vec<T>,
        bus_reader: Option<Arc<Mutex<BusReader<T>>>>,
    }

    impl<T: Clone + Sync> BusInput<T> for StubBusReader<T> {
        fn clear_input_rx(&mut self) {
            self.bus_reader = None;
        }

        fn set_input_rx(&mut self, input_tx: Arc<Mutex<BusReader<T>>>) {
            self.bus_reader = Some(input_tx);
        }
    }

    impl<T: Clone + Sync> StubBusReader<T> {
        fn new() -> Self {
            Self {
                content: vec![],
                bus_reader: None
            }
        }

        fn got_input_rx(&self) -> bool {
            self.bus_reader.is_some()
        }

        fn read(&mut self) -> Vec<T> {
            match &self.bus_reader {
                None => {
                    panic!("No bus reader set in StubBusReader");
                }
                Some(bus_reader) => {
                    loop {
                        match bus_reader.clone().lock().unwrap().recv_timeout(Duration::from_millis(500)) {
                            Ok(ke) => {
                                self.content.push(ke);
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
            self.content.clone()
        }
    }

    /*
    struct FakeSourceEncoder {
        terminate: Arc<AtomicBool>,
        input_rx: Arc<Mutex<Option<Arc<Mutex<BusReader<KeyingEvent>>>>>>,
        output_tx: Arc<Mutex<Option<Arc<Mutex<Bus<SourceEncoding>>>>>>
    }

    impl FakeSourceEncoder {
        fn new(terminate: Arc<AtomicBool>) -> Self {
            Self {
                terminate,
                input_rx: Arc::new(Mutex::new(None)),
                output_tx: Arc::new(Mutex::new(None)),
            }
        }

        fn encode(&mut self, keying_event: KeyingEvent) -> Option<SourceEncoding> {
            // todo!()
        }

        fn start_encoding(&mut self) {
            info!("Encoding loop started");
            loop {
                if self.terminate.load(Ordering::SeqCst) {
                    info!("Terminating encoding loop");
                    break;
                }

                match self.keying_event_rx.lock().unwrap().as_deref() {
                    None => {
                        // Input channel hasn't been set yet
                        thread::sleep(Duration::from_millis(100));
                    }
                    Some(input_rx) => {
                        match input_rx.lock().unwrap().recv_timeout(Duration::from_millis(100)) {
                            Ok(keying_event) => {
                                let encoding: Option<SourceEncoding> = encode(keying_event);
                                match encoding {
                                    None => {

                                    }
                                    Some(source_encoding) => {
                                        match self.output_tx.lock().unwrap().as_deref() {
                                            None => {
                                                // Output channel hasn't been set yet
                                            }
                                            Some(bus) => {
                                                bus.lock().unwrap().broadcast(source_encoding);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                // Don't log, it's just noise - timeout gives opportunity to go round loop and
                                // check for terminate.
                            }
                        }
                    }
                }
            }
            info!("Encoding loop ended");
        }
    }

    impl BusInput<KeyingEvent> for FakeSourceEncoder {
        fn clear_input_rx(&mut self) {
            match self.input_rx.lock() {
                Ok(mut locked) => { *locked = None; }
                Err(_) => {}
            }
        }

        fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<KeyingEvent>>>) {
            match self.input_rx.lock() {
                Ok(mut locked) => { *locked = Some(input_rx); }
                Err(_) => {}
            }
        }
    }

    impl BusOutput<SourceEncoding> for FakeSourceEncoder {
        fn clear_output_tx(&mut self) {
            match self.output_tx.lock() {
                Ok(mut locked) => {
                    *locked = None;
                }
                Err(_) => {}
            }
        }

        fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<SourceEncoding>>>) {
            match self.output_tx.lock() {
                Ok(mut locked) => { *locked = Some(output_tx); }
                Err(_) => {}
            }
        }
    }
*/


    #[rstest]
    #[serial]
    pub fn termination(mut fixture: ApplicationFixture) {
        assert_eq!(fixture.application.terminated(), false);
        test_util::wait_5_ms();
        fixture.application.terminate();
        test_util::wait_5_ms();
        assert_eq!(fixture.application.terminated(), true);
        assert_eq!(fixture.terminate.load(Ordering::SeqCst), true);
    }

    #[rstest]
    #[serial]
    pub fn initial_mode(fixture: ApplicationFixture) {
        assert_that!(fixture.application.get_mode(), none());
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), false);
        assert_eq!(fixture.application.got_tone_generator(), false);
        assert_eq!(fixture.application.got_tone_generator_rx(), false);
        assert_eq!(fixture.application.got_source_encoder(), false);
        assert_eq!(fixture.application.got_source_encoder_keying_event_rx(), false);
        assert_eq!(fixture.application.got_source_encoder_source_encoding_rx(), false);
    }

    #[rstest]
    #[serial]
    pub fn mode_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        assert_that!(fixture.application.get_mode(), has(ApplicationMode::KeyerDiag));
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
        assert_eq!(fixture.application.got_tone_generator(), false);
        assert_eq!(fixture.application.got_tone_generator_rx(), true);
        assert_eq!(fixture.application.got_source_encoder(), false);
        assert_eq!(fixture.application.got_source_encoder_keying_event_rx(), false);
        assert_eq!(fixture.application.got_source_encoder_source_encoding_rx(), false);
    }

    #[rstest]
    #[serial]
    pub fn mode_source_encoder_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        assert_that!(fixture.application.get_mode(), has(ApplicationMode::SourceEncoderDiag));
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), false);
        assert_eq!(fixture.application.got_tone_generator(), false);
        assert_eq!(fixture.application.got_tone_generator_rx(), true);
        assert_eq!(fixture.application.got_source_encoder(), false);
        assert_eq!(fixture.application.got_source_encoder_keying_event_rx(), true);
        assert_eq!(fixture.application.got_source_encoder_source_encoding_rx(), false);
    }



    #[rstest]
    #[serial]
    pub fn set_clear_keyer(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
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
    #[serial]
    pub fn set_clear_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
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
    #[serial]
    pub fn set_clear_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
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
    #[serial]
    pub fn set_clear_source_encoder(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        assert_eq!(fixture.application.got_source_encoder(), false);
        assert_eq!(fixture.application.got_source_encoder_keying_event_rx(), true);
        let source_encoder = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_source_encoder(source_encoder);
        assert_eq!(fixture.application.got_source_encoder(), true);
        assert_eq!(fixture.application.got_source_encoder_keying_event_rx(), true);
        fixture.application.clear_source_encoder();
        assert_eq!(fixture.application.got_source_encoder(), false);
        assert_eq!(fixture.application.got_source_encoder_keying_event_rx(), true);
    }


    // Wiring tests that check actual traffic is sent between components, and prevented after
    // unwiring. Tests use the diag ApplicationModes and check wiring/unwiring of all implicated
    // components.

    #[rstest]
    #[serial]
    // No need for the FakeKeyer and StubBusReader to have their own threads, so long as no more
    // than 16 elements are placed onto the bus.
    pub fn keyer_diag_bus_wiring(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);

        let sent_keying = vec![KeyingEvent::Start(), KeyingEvent::End()];
        let fake_keyer = Arc::new(Mutex::new(FakeKeyer::new(sent_keying.clone())));
        assert_that!(fake_keyer.lock().unwrap().got_output_tx(), false);
        let application_fake_keyer = fake_keyer.clone();
        fixture.application.set_keyer(application_fake_keyer);
        assert_that!(fake_keyer.lock().unwrap().got_output_tx(), true);

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


        fake_keyer.lock().unwrap().start_sending();
        info!("Test sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Test out of sleep");


        let tone_generator_received_keying = tone_generator.lock().unwrap().read();
        let keyer_diag_received_keying = keyer_diag.lock().unwrap().read();

        let expected_tone_generator_received_keying = vec![
            KeyingEventToneChannel { keying_event: KeyingEvent::Start(), tone_channel: 0 as ToneChannel},
            KeyingEventToneChannel { keying_event: KeyingEvent::End(), tone_channel: 0 as ToneChannel} ];

        assert_eq!(tone_generator_received_keying, expected_tone_generator_received_keying);
        assert_eq!(keyer_diag_received_keying, sent_keying.clone());
    }

    #[rstest]
    #[serial]
    pub fn keyer_diag_bus_unwiring(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);

        let fake_keyer = Arc::new(Mutex::new(FakeKeyer::new(vec![])));
        let application_fake_keyer = fake_keyer.clone();
        fixture.application.set_keyer(application_fake_keyer);

        let tone_generator = Arc::new(Mutex::new(StubBusReader::new()));
        let application_tone_generator = tone_generator.clone();
        fixture.application.set_tone_generator(application_tone_generator);

        let keyer_diag = Arc::new(Mutex::new(StubBusReader::new()));
        let application_keyer_diag = keyer_diag.clone();
        fixture.application.set_keyer_diag(application_keyer_diag);


        fake_keyer.lock().unwrap().start_sending();
        info!("Test sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Test out of sleep");

        fixture.application.clear_keyer();
        assert_eq!(fake_keyer.lock().unwrap().got_output_tx(), false);
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


    #[rstest]
    #[serial]
    // No need for the FakeKeyer and StubBusReader to have their own threads, so long as no more
    // than 16 elements are placed onto the bus.
    pub fn source_encoder_diag_bus_wiring(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);

        let sent_keying = vec![KeyingEvent::Start(), KeyingEvent::End()];
        let fake_keyer = Arc::new(Mutex::new(FakeKeyer::new(sent_keying.clone())));
        assert_that!(fake_keyer.lock().unwrap().got_output_tx(), false);
        let application_fake_keyer = fake_keyer.clone();
        fixture.application.set_keyer(application_fake_keyer);
        assert_that!(fake_keyer.lock().unwrap().got_output_tx(), true);

        let tone_generator = Arc::new(Mutex::new(StubBusReader::new()));
        assert_that!(tone_generator.lock().unwrap().got_input_rx(), false);
        let application_tone_generator = tone_generator.clone();
        fixture.application.set_tone_generator(application_tone_generator);
        assert_that!(tone_generator.lock().unwrap().got_input_rx(), true);

        // Use the real SourceEncoder
        let mut se = SourceEncoder::new(fixture.application.terminate_flag(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        se.set_keyer_speed(12 as KeyerSpeed);
        let source_encoder = Arc::new(Mutex::new(se));
        fixture.application.set_source_encoder(source_encoder);

        // The SourceEncoder 'diag' is a delayed bus that then decodes, and passes the decoded data
        // to Playback. For this test, just read/write with no delay.

        // let mut delayed_bus = DelayedBus::new(fixture.application.terminate_flag(), fixture.scheduled_thread_pool.clone(), Duration::from_millis(10));


        fake_keyer.lock().unwrap().start_sending();
        info!("Test sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Test out of sleep");

        //
        // let tone_generator_received_keying = tone_generator.lock().unwrap().read();
        // let keyer_diag_received_keying = keyer_diag.lock().unwrap().read();
        //
        // let expected_tone_generator_received_keying = vec![
        //     KeyingEventToneChannel { keying_event: KeyingEvent::Start(), tone_channel: 0 as ToneChannel},
        //     KeyingEventToneChannel { keying_event: KeyingEvent::End(), tone_channel: 0 as ToneChannel} ];
        //
        // assert_eq!(tone_generator_received_keying, expected_tone_generator_received_keying);
        // assert_eq!(keyer_diag_received_keying, sent_keying.clone());

    }

    #[rstest]
    #[serial]
    // No need for the FakeKeyer and StubBusReader to have their own threads, so long as no more
    // than 16 elements are placed onto the bus.
    pub fn source_encoder_diag_bus_unwiring(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);

    }
}
