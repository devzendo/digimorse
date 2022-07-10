/*
 * These tests are manually invoked and validate that the application provides a suitably
 * performant backbone for the rest of the system.
 * They assume the existence of a valid configuration file as created by main.
 */
#[cfg(test)]
mod diag_application_spec {
    use std::env;
    use std::fs::File;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use bus::{Bus, BusReader};
    use csv::Writer;

    use log::{debug, info, warn};
    use portaudio as pa;
    use rstest::*;
    use pretty_hex::*;
    use syncbox::ScheduledThreadPool;

    use crate::libs::application::application::{Application, ApplicationMode, BusInput};
    use crate::libs::audio::tone_generator::ToneGenerator;
    use crate::libs::config_dir::config_dir;
    use crate::libs::config_file::config_file::ConfigurationStore;
    use crate::libs::delayed_bus::delayed_bus::DelayedBus;
    use crate::libs::application::application::BusOutput;
    use crate::libs::keyer_io::arduino_keyer_io::ArduinoKeyer;
    use crate::libs::keyer_io::keyer_io::{Keyer, KeyerSpeed, KeyingEvent};
    use crate::libs::playback::playback::Playback;
    use crate::libs::serial_io::serial_io::DefaultSerialIO;
    use crate::libs::source_codec::source_decoder::SourceDecoder;
    use crate::libs::source_codec::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, SourceEncoding};
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

        let pa = pa::PortAudio::new();
        if pa.is_err() {
            panic!("Portaudio setup failure: {:?}", pa);
        }

        let application = Application::new(terminate.clone(), scheduled_thread_pool.clone(), pa.unwrap());

        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        DiagApplicationFixture {
            config,
            terminate,
            _scheduled_thread_pool: scheduled_thread_pool,
            application,
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

    fn set_tone_generator(_config: &mut ConfigurationStore, application: &mut Application) -> Arc<Mutex<ToneGenerator>> {
        info!("Setting up tone generator");
        let old_macbook = false;
        let out_dev_str = if old_macbook {"Built-in Output"} else {"MacBook Pro Speakers"};
        let output_settings = application.open_output_audio_device(out_dev_str);
        match output_settings {
            Ok(_) => {}
            Err(err) => {
                panic!("Can't obtain output settings for {}: {}", out_dev_str, err);
            }
        }

        let sidetone_frequency = 600 as u16;
        // let tone_generator_keying_event_tone_channel_rx = Arc::new(Mutex::new(keying_event_tone_channel_rx));
        let mut tone_generator = ToneGenerator::new(sidetone_frequency,
                                                    application.terminate_flag());

        info!("Setting audio frequency...");
        tone_generator.set_audio_frequency(0, sidetone_frequency);

        match tone_generator.start_callback(application.pa_ref(), output_settings.unwrap()) { // also initialises DDS for sidetone.
            Ok(_) => {}
            Err(err) => {
                panic!("Can't initialise tone generator callback: {}", err);
            }
        }
        let return_tone_generator = Arc::new(Mutex::new(tone_generator));
        application.set_tone_generator(return_tone_generator.clone());
        return return_tone_generator;
    }

    impl Drop for DiagApplicationFixture {
        fn drop(&mut self) {
            debug!("ApplicationFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("ApplicationFixture ...set terminate flag");
        }
    }

    fn set_keyer_diag(_config: &mut ConfigurationStore, application: &mut Application) -> Arc<Mutex<KeyerDiag>> {
        let keyer_diag = KeyerDiag::new("keying.csv", application.terminate_flag());
        let application_keyer_diag = Arc::new(Mutex::new(keyer_diag));
        let return_keyer_diag = application_keyer_diag.clone();
        application.set_keyer_diag(application_keyer_diag);
        return return_keyer_diag;
    }

    struct KeyerDiag {
        bus_reader: Option<Arc<Mutex<BusReader<KeyingEvent>>>>,
        csv_writer: Writer<File>,
        terminate: Arc<AtomicBool>,
    }

    impl KeyerDiag {
        fn new(csv_path: &str, terminate: Arc<AtomicBool>) -> Self {
            let wtr = Writer::from_path(csv_path);
            match wtr {
                Ok(_) => {}
                Err(err) => {
                    panic!("Can't initialise CSV writer to path {}: {}", csv_path, err);
                }
            }
            Self {
                bus_reader: None,
                csv_writer: wtr.unwrap(),
                terminate
            }
        }

        // Precondition: set_input_rx has been called.
        fn process(&mut self) {
            loop {
                if self.terminate.load(Ordering::SeqCst) {
                    break;
                }
                let result = self.bus_reader.as_deref().unwrap().lock().unwrap().recv_timeout(Duration::from_millis(250));
                match result {
                    Ok(keying_event) => {
                        info!("KeyerDiag: Keying Event {}", keying_event);
                        match keying_event {
                            // KeyingTimedEvents give the duration at the END of a (mark|space). If the
                            // key is now up, then we've just heard a mark (key down), and if it's now down,
                            // we've just heard a space (key up).
                            // If we see a start, that's just the starting key down edge of a mark; an
                            // end is actually meaningless in terms of keying - it's just a timeout after
                            // the user has ended keying. In terms of generating a histogram of
                            // keying, the stream should be a single long over - ie no END/STARTs in the
                            // middle - otherwise you'll see two consecutive MARKs, which makes no sense.
                            KeyingEvent::Timed(timed) => {
                                self.csv_writer.write_record(&[if timed.up { "MARK" } else { "SPACE" }, format!("{}", timed.duration).as_str()]).unwrap();
                                self.csv_writer.flush().unwrap();
                            }
                            KeyingEvent::Start() => {}
                            KeyingEvent::End() => {}
                        }
                    }
                    Err(_) => {
                        // be quiet, it's ok..
                    }
                }
            }
        }
    }

    impl BusInput<KeyingEvent> for KeyerDiag {
        fn clear_input_rx(&mut self) {
            self.bus_reader = None;
        }

        fn set_input_rx(&mut self, input_tx: Arc<Mutex<BusReader<KeyingEvent>>>) {
            self.bus_reader = Some(input_tx);
        }
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn mode_keyer_diag(mut fixture: DiagApplicationFixture) {
        debug!("start mode_keyer_diag");
        fixture.application.set_mode(ApplicationMode::KeyerDiag);
        set_keyer(&mut fixture.config, &mut fixture.application);
        set_tone_generator(&mut fixture.config, &mut fixture.application);
        let keyer_diag = set_keyer_diag(&mut fixture.config, &mut fixture.application);
        debug!("processing keyer_diag");
        keyer_diag.lock().unwrap().process();
        debug!("end mode_keyer_diag");
    }

    struct SourceEncoderDiag {
        delayed_source_encoding_bus: Arc<Mutex<Bus<SourceEncoding>>>,
        delayed_source_encoding_bus_rx: BusReader<SourceEncoding>,
        delayed_bus: DelayedBus<SourceEncoding>,
        terminate: Arc<AtomicBool>,
        playback: Arc<Mutex<Playback>>,
        replay_sidetone_frequency: u16,
    }
    impl BusInput<SourceEncoding> for SourceEncoderDiag {
        fn clear_input_rx(&mut self) {
            self.delayed_bus.clear_input_rx();
        }

        fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<SourceEncoding>>>) {
            self.delayed_bus.set_input_rx(input_rx);
        }
    }
    impl SourceEncoderDiag {
        fn new(terminate: Arc<AtomicBool>, scheduled_thread_pool: Arc<ScheduledThreadPool>, playback: Arc<Mutex<Playback>>, replay_sidetone_frequency: u16) -> Self {
            let mut delayed_source_encoding_bus = Bus::new(16);
            let delayed_source_encoding_bus_rx = delayed_source_encoding_bus.add_rx();
            let mut delayed_bus: DelayedBus<SourceEncoding> = DelayedBus::new(
                terminate.clone(),
                scheduled_thread_pool.clone(),
                Duration::from_secs(10));
            let shared_delayed_source_encoding_bus = Arc::new(Mutex::new(delayed_source_encoding_bus));
            delayed_bus.set_output_tx(shared_delayed_source_encoding_bus.clone());
            Self {
                delayed_source_encoding_bus: shared_delayed_source_encoding_bus,
                delayed_source_encoding_bus_rx,
                delayed_bus,
                terminate,
                playback,
                replay_sidetone_frequency,
            }
        }

        // Precondition: set_input_rx has been called.
        fn process(&mut self) {
            const REPLAY_CALLSIGN_HASH: u16 = 0x1234u16;
            let source_decoder = SourceDecoder::new(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);

            loop {
                if self.terminate.load(Ordering::SeqCst) {
                    break;
                }
                let result = self.delayed_source_encoding_bus_rx.recv_timeout(Duration::from_millis(250));
                match result {
                    Ok(source_encoding) => {
                        info!("SourceEncoderDiag: Source Encoding {}", source_encoding);
                        debug!("SourceEncodingDiag: isEnd {}", source_encoding.is_end);
                        let hexdump = pretty_hex(&source_encoding.block);
                        let hexdump_lines = hexdump.split("\n");
                        for line in hexdump_lines {
                            debug!("SourceEncodingDiag: Encoding {}", line);
                        }
                        // The SourceEncoding can now be decoded...
                        let source_decode_result = source_decoder.source_decode(source_encoding.block);
                        if source_decode_result.is_ok() {
                            // The decoded frames can now be played back (using another tone generator
                            // channel, at the replay sidetone audio frequency).
                            self.playback.lock().unwrap().play(source_decode_result, REPLAY_CALLSIGN_HASH, self.replay_sidetone_frequency);
                        } else {
                            warn!("Error from source decoder: {:?}", source_decode_result);
                        }
                    }
                    Err(_) => {
                        // be quiet, it's ok..
                    }
                }
            }
        }
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn mode_source_encoder_diag(mut fixture: DiagApplicationFixture) {
        // Keying goes into the Application's keying_event_bus. The SourceEncoder reads from this.
        // This bus is also read via the Application's embedded TransformBus
        // that adds channel 0 to the KeyingEvents and emits them to the ToneGenerator.
        // The SourceEncoder emits SourceEncodings to the Application's source_encoder_bus.
        // This diag attaches a DelayedBus to the Application's source_encoder_diag, so it receives
        // the SourceEncodings. The output of the diag (the SourceEncodings) are then decoded, and
        // submitted to the Playback via method calls. This allocates channels of the ToneGenerator
        // via method calls, and also submits KeyingEventToneChannel events to the ToneGenerator,
        // which plays them.
        debug!("start mode_source_encoder_diag");
        fixture.application.set_mode(ApplicationMode::SourceEncoderDiag);
        set_keyer(&mut fixture.config, &mut fixture.application);
        let tone_generator = set_tone_generator(&mut fixture.config, &mut fixture.application);

        // The source_encoder_diag doesn't use a bus to communicate to playback - it's done by method
        // calls.
        // Playback uses method calls to tone_generator to allocate/deallocate channels, but the tones
        // on those channels are sent to the tone_generator over a bus.

        // TODO
        // fixture.application.set_source_encoder_diag(Arc::new(Mutex::new(delayed_bus))); // the delayed_bus input is the source_encoder_diag_rx, in SourceEncoderDiag mode.

        let playback = Playback::new(fixture.application.terminate_flag(), fixture.application.scheduled_thread_pool(),
                                         tone_generator);
        let application_playback = Arc::new(Mutex::new(playback));
        let source_encoder_diag_playback = application_playback.clone();
        fixture.application.set_playback(application_playback);

        let source_encoder_diag = SourceEncoderDiag::new(
            fixture.application.terminate_flag(),
            fixture.application.scheduled_thread_pool(),
            source_encoder_diag_playback,
            fixture.config.get_sidetone_frequency() + 50);
        let shared_source_encoder_diag = Arc::new(Mutex::new(source_encoder_diag));
        let diag_shared_source_encoder_diag = shared_source_encoder_diag.clone();
        fixture.application.set_source_encoder_diag(shared_source_encoder_diag);

        test_util::wait_n_ms(1000);

        debug!("processing source_encoder_diag");
        diag_shared_source_encoder_diag.lock().unwrap().process();
        debug!("end mode_source_encoder_diag");
    }
}
