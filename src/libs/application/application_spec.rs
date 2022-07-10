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

    use crate::libs::application::application::{Application, ApplicationMode, BusInput, BusOutput};
    use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneChannel};
    use crate::libs::keyer_io::keyer_io::KeyingEvent;
    use crate::libs::source_codec::source_encoding::{Frame, SourceEncoding};
    use crate::libs::source_codec::test_encoding_builder::encoded;
    use crate::libs::util::test_util;

    const TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS: usize = 64;

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

    struct StubBusWriter<T> {
        bus_writer: Option<Arc<Mutex<Bus<T>>>>,
    }

    impl<T: Clone + Sync> BusOutput<T> for StubBusWriter<T> {
        fn clear_output_tx(&mut self) {
            self.bus_writer = None;
        }

        fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<T>>>) {
            self.bus_writer = Some(output_tx);
        }
    }

    impl<T: Clone + Sync> StubBusWriter<T> {
        fn new() -> Self {
            Self {
                bus_writer: None
            }
        }

        fn got_output_tx(&self) -> bool {
            self.bus_writer.is_some()
        }

        fn write(&mut self, data: Vec<T>) {
            match &self.bus_writer {
                None => {
                    warn!("No bus writer set in StubBusWriter");
                }
                Some(bus_writer) => {
                    for v in data {
                        bus_writer.lock().unwrap().broadcast(v);
                    }
                }
            }
            info!("Out of StubBusWriter write");
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
                    warn!("No bus reader set in StubBusReader");
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


    struct StubBusReaderWriter<I, O> {
        content: Vec<I>,
        bus_reader: Option<Arc<Mutex<BusReader<I>>>>,
        bus_writer: Option<Arc<Mutex<Bus<O>>>>,
    }

    impl<I: Clone + Sync, O: Clone + Sync> BusInput<I> for StubBusReaderWriter<I, O> {
        fn clear_input_rx(&mut self) {
            self.bus_reader = None;
        }

        fn set_input_rx(&mut self, input_tx: Arc<Mutex<BusReader<I>>>) {
            self.bus_reader = Some(input_tx);
        }
    }

    impl<I: Clone + Sync, O: Clone + Sync> BusOutput<O> for StubBusReaderWriter<I, O> {
        fn clear_output_tx(&mut self) {
            self.bus_writer = None;
        }

        fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<O>>>) {
            self.bus_writer = Some(output_tx);
        }
    }

    impl<I: Clone + Sync, O: Clone + Sync> StubBusReaderWriter<I, O> {
        fn new() -> Self {
            Self {
                content: vec![],
                bus_reader: None,
                bus_writer: None
            }
        }

        fn got_output_tx(&self) -> bool {
            self.bus_writer.is_some()
        }

        fn got_input_rx(&self) -> bool {
            self.bus_reader.is_some()
        }

        fn read(&mut self) -> Vec<I> {
            match &self.bus_reader {
                None => {
                    warn!("No bus reader set in StubBusReaderWriter");
                }
                Some(bus_reader) => {
                    loop {
                        match bus_reader.clone().lock().unwrap().recv_timeout(Duration::from_millis(500)) {
                            Ok(ke) => {
                                self.content.push(ke);
                            }
                            Err(_) => {
                                info!("StubBusReaderWriter timed out on read");
                                break;
                            }
                        }
                    }
                }
            }
            info!("Out of StubBusReaderWriter read");
            self.content.clone()
        }

        fn write(&mut self, data: Vec<O>) {
            match &self.bus_writer {
                None => {
                    warn!("No bus writer set in StubBusWriter");
                }
                Some(bus_writer) => {
                    for v in data {
                        bus_writer.lock().unwrap().broadcast(v);
                    }
                }
            }
            info!("Out of StubBusReaderWriter write");
        }
    }



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

    // Which bus attachment points are present?

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
        assert_eq!(fixture.application.got_source_encoder_diag_source_encoding_rx(), false);
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
        assert_eq!(fixture.application.got_source_encoder_diag_source_encoding_rx(), true);
    }


    #[rstest]
    #[serial]
    pub fn set_clear_keyer(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        assert_eq!(fixture.application.got_keyer(), false);
        assert_eq!(fixture.application.got_keyer_diag_rx(), true);
        let keyer = Arc::new(Mutex::new(StubBusWriter::new()));
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
        let source_encoder: Arc<Mutex<StubBusReaderWriter<KeyingEvent, SourceEncoding>>> = Arc::new(Mutex::new(StubBusReaderWriter::new()));
        fixture.application.set_source_encoder(source_encoder);
        assert_eq!(fixture.application.got_source_encoder(), true);
        assert_eq!(fixture.application.got_source_encoder_keying_event_rx(), true);
        fixture.application.clear_source_encoder();
        assert_eq!(fixture.application.got_source_encoder(), false);
        assert_eq!(fixture.application.got_source_encoder_keying_event_rx(), true);
    }

    #[rstest]
    #[serial]
    pub fn set_clear_source_encoder_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        assert_eq!(fixture.application.got_source_encoder_diag(), false);
        assert_eq!(fixture.application.got_source_encoder_diag_source_encoding_rx(), true);
        let source_encoder_diag = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_source_encoder_diag(source_encoder_diag);
        assert_eq!(fixture.application.got_source_encoder_diag(), true);
        assert_eq!(fixture.application.got_source_encoder_diag_source_encoding_rx(), true);
        fixture.application.clear_source_encoder_diag();
        assert_eq!(fixture.application.got_source_encoder_diag(), false);
        assert_eq!(fixture.application.got_source_encoder_diag_source_encoding_rx(), true);
    }

    #[rstest]
    #[serial]
    pub fn set_clear_playback(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        assert_eq!(fixture.application.got_playback(), false);
        let playback = Arc::new(Mutex::new(StubBusWriter::new()));
        fixture.application.set_playback(playback);
        assert_eq!(fixture.application.got_playback(), true);
        fixture.application.clear_playback();
        assert_eq!(fixture.application.got_playback(), false);
    }

    // Mode/Component set/clear validation tests

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set keyer in mode None")]
    pub fn none_cannot_set_keyer(mut fixture: ApplicationFixture) {
        let keyer: Arc<Mutex<StubBusWriter<KeyingEvent>>> = Arc::new(Mutex::new(StubBusWriter::new()));
        fixture.application.set_keyer(keyer);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear keyer in mode None")]
    pub fn none_cannot_clear_keyer(mut fixture: ApplicationFixture) {
        fixture.application.clear_keyer();
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set tone_generator in mode None")]
    pub fn none_cannot_set_tone_generator(mut fixture: ApplicationFixture) {
        let tone_generator: Arc<Mutex<StubBusReader<KeyingEventToneChannel>>> = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_tone_generator(tone_generator);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear tone_generator in mode None")]
    pub fn none_mode_cannot_clear_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.clear_tone_generator();
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set keyer_diag in mode None")]
    pub fn none_mode_cannot_set_keyer_diag(mut fixture: ApplicationFixture) {
        let keyer_diag: Arc<Mutex<StubBusReader<KeyingEvent>>> = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_keyer_diag(keyer_diag);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear keyer_diag in mode None")]
    pub fn none_mode_cannot_clear_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.clear_keyer_diag();
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set source_encoder in mode None")]
    pub fn none_mode_cannot_set_source_encoder(mut fixture: ApplicationFixture) {
        let source_encoder: Arc<Mutex<StubBusReaderWriter<KeyingEvent, SourceEncoding>>> = Arc::new(Mutex::new(StubBusReaderWriter::new()));
        fixture.application.set_source_encoder(source_encoder);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear source_encoder in mode None")]
    pub fn none_mode_cannot_clear_source_encoder(mut fixture: ApplicationFixture) {
        fixture.application.clear_source_encoder();
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set source_encoder_diag in mode None")]
    pub fn none_mode_cannot_set_source_encoder_diag(mut fixture: ApplicationFixture) {
        let source_encoder_diag: Arc<Mutex<StubBusReader<SourceEncoding>>> = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_source_encoder_diag(source_encoder_diag);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear source_encoder_diag in mode None")]
    pub fn none_mode_cannot_clear_source_encoder_diag(mut fixture: ApplicationFixture) {
        fixture.application.clear_source_encoder_diag();
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set playback in mode None")]
    pub fn none_mode_cannot_set_playback(mut fixture: ApplicationFixture) {
        let playback: Arc<Mutex<StubBusWriter<KeyingEventToneChannel>>> = Arc::new(Mutex::new(StubBusWriter::new()));
        fixture.application.set_playback(playback);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear playback in mode None")]
    pub fn none_mode_cannot_clear_playback(mut fixture: ApplicationFixture) {
        fixture.application.clear_playback();
    }


    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set source_encoder in mode Some(KeyerDiag)")]
    pub fn keyer_diag_mode_cannot_set_source_encoder(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        let source_encoder: Arc<Mutex<StubBusReaderWriter<KeyingEvent, SourceEncoding>>> = Arc::new(Mutex::new(StubBusReaderWriter::new()));
        fixture.application.set_source_encoder(source_encoder);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear source_encoder in mode Some(KeyerDiag)")]
    pub fn keyer_diag_mode_cannot_clear_source_encoder(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        fixture.application.clear_source_encoder();
    }


    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set source_encoder_diag in mode Some(KeyerDiag)")]
    pub fn keyer_diag_mode_cannot_set_source_encoder_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        let source_encoder_diag: Arc<Mutex<StubBusReader<SourceEncoding>>> = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_source_encoder_diag(source_encoder_diag);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear source_encoder_diag in mode Some(KeyerDiag)")]
    pub fn keyer_diag_mode_cannot_clear_source_encoder_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        fixture.application.clear_source_encoder_diag();
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set playback in mode Some(KeyerDiag)")]
    pub fn keyer_diag_mode_cannot_set_playback(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        let playback: Arc<Mutex<StubBusWriter<KeyingEventToneChannel>>> = Arc::new(Mutex::new(StubBusWriter::new()));
        fixture.application.set_playback(playback);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear playback in mode Some(KeyerDiag)")]
    pub fn keyer_diag_mode_cannot_clear_playback(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        fixture.application.clear_playback();
    }


    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set keyer_diag in mode Some(SourceEncoderDiag)")]
    pub fn source_encoder_diag_mode_cannot_set_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        let keyer_diag: Arc<Mutex<StubBusReader<KeyingEvent>>> = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_keyer_diag(keyer_diag);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear keyer_diag in mode Some(SourceEncoderDiag)")]
    pub fn source_encoder_diag_mode_cannot_clear_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        fixture.application.clear_keyer_diag();
    }


    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set keyer_diag in mode Some(Full)")]
    pub fn full_mode_cannot_set_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::Full);
        let keyer_diag: Arc<Mutex<StubBusReader<KeyingEvent>>> = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_keyer_diag(keyer_diag);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear keyer_diag in mode Some(Full)")]
    pub fn full_mode_cannot_clear_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::Full);
        fixture.application.clear_keyer_diag();
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't set source_encoder_diag in mode Some(Full)")]
    pub fn full_mode_cannot_set_source_encoder_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::Full);
        let source_encoder_diag: Arc<Mutex<StubBusReader<SourceEncoding>>> = Arc::new(Mutex::new(StubBusReader::new()));
        fixture.application.set_source_encoder_diag(source_encoder_diag);
    }

    #[rstest]
    #[serial]
    #[should_panic(expected="Can't clear source_encoder_diag in mode Some(Full)")]
    pub fn full_mode_cannot_clear_source_encoder_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::Full);
        fixture.application.clear_source_encoder_diag();
    }




    // Wiring tests that check actual traffic is sent between components, and prevented after
    // unwiring. Tests use the diag ApplicationModes and check wiring/unwiring of all implicated
    // components.


    #[rstest]
    #[serial]
    pub fn full_mode_keyer_sends_to_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::Full);
        _keyer_sends_to_tone_generator(fixture);
    }

    #[rstest]
    #[serial]
    pub fn keyer_diag_mode_keyer_sends_to_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        _keyer_sends_to_tone_generator(fixture);
    }

    #[rstest]
    #[serial]
    pub fn source_encoder_diag_mode_keyer_sends_to_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        _keyer_sends_to_tone_generator(fixture);
    }

    fn _keyer_sends_to_tone_generator(mut fixture: ApplicationFixture) {
        let keyer: Arc<Mutex<StubBusWriter<KeyingEvent>>> = Arc::new(Mutex::new(StubBusWriter::new()));
        let test_keyer = keyer.clone();
        fixture.application.set_keyer(keyer);

        // Goes via KeyingEvent bus through the TransformBus to the ToneGenerator, as a
        // KeyingEventToneChannel event.

        let tone_generator: Arc<Mutex<StubBusReader<KeyingEventToneChannel>>> = Arc::new(Mutex::new(StubBusReader::new()));
        let test_tone_generator = tone_generator.clone();
        fixture.application.set_tone_generator(tone_generator);

        test_util::wait_5_ms();

        let sent_keying = vec![KeyingEvent::Start(), KeyingEvent::End()];

        test_keyer.lock().unwrap().write(sent_keying);

        let tone_generator_received_keying = test_tone_generator.lock().unwrap().read();

        let expected_tone_generator_received_keying = vec![
            KeyingEventToneChannel { keying_event: KeyingEvent::Start(), tone_channel: 0 as ToneChannel},
            KeyingEventToneChannel { keying_event: KeyingEvent::End(), tone_channel: 0 as ToneChannel} ];

        assert_eq!(tone_generator_received_keying, expected_tone_generator_received_keying);
    }

    #[rstest]
    #[serial]
    pub fn keyer_diag_mode_keyer_sends_to_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        let keyer: Arc<Mutex<StubBusWriter<KeyingEvent>>> = Arc::new(Mutex::new(StubBusWriter::new()));
        let test_keyer = keyer.clone();
        fixture.application.set_keyer(keyer);

        // Goes via KeyingEvent bus through the TransformBus to the ToneGenerator, as a
        // KeyingEventToneChannel event.

        let keyer_diag: Arc<Mutex<StubBusReader<KeyingEvent>>> = Arc::new(Mutex::new(StubBusReader::new()));
        let test_keyer_diag = keyer_diag.clone();
        fixture.application.set_keyer_diag(keyer_diag);

        test_util::wait_5_ms();

        let sent_keying = vec![KeyingEvent::Start(), KeyingEvent::End()];
        let test_sent_keying = sent_keying.clone();

        test_keyer.lock().unwrap().write(sent_keying);

        let keyer_diag_received_keying = test_keyer_diag.lock().unwrap().read();

        assert_eq!(keyer_diag_received_keying, test_sent_keying);
    }

    #[rstest]
    #[serial]
    pub fn keyer_diag_mode_clear_keyer_prevents_send_to_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        _keyer_does_not_send_to_tone_generator(fixture);
    }

    #[rstest]
    #[serial]
    pub fn source_encoder_diag_mode_clear_keyer_prevents_send_to_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        _keyer_does_not_send_to_tone_generator(fixture);
    }

    fn _keyer_does_not_send_to_tone_generator(mut fixture: ApplicationFixture) {
        let keyer: Arc<Mutex<StubBusWriter<KeyingEvent>>> = Arc::new(Mutex::new(StubBusWriter::new()));
        let test_keyer = keyer.clone();
        fixture.application.set_keyer(keyer);
        fixture.application.clear_keyer();

        // Goes via KeyingEvent bus through the TransformBus to the ToneGenerator, as a
        // KeyingEventToneChannel event.

        let tone_generator: Arc<Mutex<StubBusReader<KeyingEventToneChannel>>> = Arc::new(Mutex::new(StubBusReader::new()));
        let test_tone_generator = tone_generator.clone();
        fixture.application.set_tone_generator(tone_generator);

        test_util::wait_5_ms();

        let sent_keying = vec![KeyingEvent::Start(), KeyingEvent::End()];

        test_keyer.lock().unwrap().write(sent_keying);

        let tone_generator_received_keying = test_tone_generator.lock().unwrap().read();

        assert_eq!(tone_generator_received_keying.len(), 0);
    }

    #[rstest]
    #[serial]
    pub fn keyer_diag_mode_clear_keyer_does_not_send_to_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        let keyer: Arc<Mutex<StubBusWriter<KeyingEvent>>> = Arc::new(Mutex::new(StubBusWriter::new()));
        let test_keyer = keyer.clone();
        fixture.application.set_keyer(keyer);
        fixture.application.clear_keyer();

        // Goes via KeyingEvent bus through the TransformBus to the ToneGenerator, as a
        // KeyingEventToneChannel event.

        let keyer_diag: Arc<Mutex<StubBusReader<KeyingEvent>>> = Arc::new(Mutex::new(StubBusReader::new()));
        let test_keyer_diag = keyer_diag.clone();
        fixture.application.set_keyer_diag(keyer_diag);

        test_util::wait_5_ms();

        let sent_keying = vec![KeyingEvent::Start(), KeyingEvent::End()];

        test_keyer.lock().unwrap().write(sent_keying);

        let keyer_diag_received_keying = test_keyer_diag.lock().unwrap().read();

        assert_eq!(keyer_diag_received_keying.len(), 0);
    }



    // Source Encoder



    #[rstest]
    #[serial]
    pub fn source_encoder_diag_mode_source_encoder_sends_to_source_encoder_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        let source_encoder: Arc<Mutex<StubBusReaderWriter<KeyingEvent, SourceEncoding>>> = Arc::new(Mutex::new(StubBusReaderWriter::new()));
        let test_source_encoder = source_encoder.clone();
        fixture.application.set_source_encoder(source_encoder);

        let source_encoder_diag: Arc<Mutex<StubBusReader<SourceEncoding>>> = Arc::new(Mutex::new(StubBusReader::new()));
        let test_source_encoder_diag = source_encoder_diag.clone();
        fixture.application.set_source_encoder_diag(source_encoder_diag);

        test_util::wait_5_ms();

        let keying_frames = &[
            Frame::WPMPolarity { wpm: 5, polarity: true },
            Frame::KeyingDeltaDah { delta: 5 },
            Frame::WPMPolarity { wpm: 60, polarity: true },
            Frame::KeyingDeltaDah { delta: 5 },
            Frame::Extension, // It stands out as 1111 in the debug output below.
            Frame::Padding
        ];
        let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, keying_frames);
        let source_encoding = SourceEncoding{ block, is_end: true };
        let vec_source_encoding = vec![source_encoding];
        let test_vec_source_encoding = vec_source_encoding.clone();

        test_source_encoder.lock().unwrap().write(vec_source_encoding);

        let source_encoder_diag_received = test_source_encoder_diag.lock().unwrap().read();

        assert_eq!(source_encoder_diag_received, test_vec_source_encoding);
    }

    // The source_encoder_diag doesn't use a bus to communicate to playback - it's done by method
    // calls.
    // Playback uses method calls to tone_generator to allocate/deallocate channels, but the tones
    // on those channels are sent to the tone_generator over a bus.

    #[rstest]
    #[serial]
    pub fn source_encoder_diag_mode_playback_sends_to_tone_generator(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);

        let playback: Arc<Mutex<StubBusWriter<KeyingEventToneChannel>>> = Arc::new(Mutex::new(StubBusWriter::new()));
        let test_playback = playback.clone();
        fixture.application.set_playback(playback);

        let tone_generator: Arc<Mutex<StubBusReader<KeyingEventToneChannel>>> = Arc::new(Mutex::new(StubBusReader::new()));
        let test_tone_generator = tone_generator.clone();
        fixture.application.set_tone_generator(tone_generator);

        test_util::wait_5_ms();

        let sent_keying_tones = vec![
            KeyingEventToneChannel{ keying_event: KeyingEvent::Start(), tone_channel: 0 },
            KeyingEventToneChannel{ keying_event: KeyingEvent::End(), tone_channel: 0}];

        test_playback.lock().unwrap().write(sent_keying_tones);

        test_util::wait_5_ms();

        let tone_generator_received_keying = test_tone_generator.lock().unwrap().read();

        let expected_tone_generator_received_keying = vec![
            KeyingEventToneChannel { keying_event: KeyingEvent::Start(), tone_channel: 0 as ToneChannel},
            KeyingEventToneChannel { keying_event: KeyingEvent::End(), tone_channel: 0 as ToneChannel} ];

        assert_eq!(tone_generator_received_keying, expected_tone_generator_received_keying);
    }
}
