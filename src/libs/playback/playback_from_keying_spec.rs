extern crate hamcrest2;


#[cfg(test)]
mod playback_from_keying_spec {
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use bus::{Bus, BusReader};
    use csv::{ReaderBuilder, StringRecord};
    use log::{debug, info};
    use portaudio as pa;
    use portaudio::PortAudio;
    use rstest::*;
    use syncbox::{ScheduledThreadPool, Task};
    use crate::libs::application::application::BusInput;
    use crate::libs::audio::audio_devices::open_output_audio_device;
    use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneGenerator};
    use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyerSpeed, KeyingEvent, KeyingTimedEvent};
    use crate::libs::playback::playback::Playback;
    use crate::libs::source_codec::source_decoder::SourceDecoder;
    use crate::libs::source_codec::source_encoder::SourceEncoder;
    use crate::libs::source_codec::source_encoding::SourceEncoding;
    use crate::libs::util::test_util;

    const TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS: usize = 64;

    pub struct PlaybackFixture {
        terminate: Arc<AtomicBool>,
        scheduled_thread_pool: Arc<ScheduledThreadPool>,
        keying_event_tx: Arc<Mutex<Bus<KeyingEvent>>>,
        source_encoder_rx: BusReader<SourceEncoding>,
        _source_encoder: SourceEncoder,
        source_decoder: SourceDecoder,
        tone_generator: Arc<Mutex<ToneGenerator>>,
        pa: Arc<PortAudio>,
        playback: Playback,
    }

    #[fixture]
    fn fixture() -> PlaybackFixture {
        info!("starting fixture");
        let terminate = Arc::new(AtomicBool::new(false));
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());
        let fixture_scheduled_thread_pool = scheduled_thread_pool.clone();
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        let mut source_encoder_tx = Bus::new(16);
        let source_encoder_rx = source_encoder_tx.add_rx();
        let mut source_encoder = SourceEncoder::new(keying_event_rx, source_encoder_tx, terminate.clone(), TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        source_encoder.set_keyer_speed(20 as KeyerSpeed);

        let source_decoder = SourceDecoder::new(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);

        let keying_event_tone_channel_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>> = Arc::new(Mutex::new(Bus::new(16)));
        let keying_event_tone_channel_rx = keying_event_tone_channel_tx.lock().unwrap().add_rx();

        let dev = "Built-in Output"; // "MacBook Pro Speakers";
        let sidetone_frequency = 600 as u16;
        info!("Instantiating tone generator...");
        let tone_generator_keying_event_tone_channel_rx = Arc::new(Mutex::new(keying_event_tone_channel_rx));
        let mut tone_generator = ToneGenerator::new(sidetone_frequency,
                                                    terminate.clone());
        tone_generator.set_input_rx(tone_generator_keying_event_tone_channel_rx);

        info!("Setting audio freqency...");
        tone_generator.set_audio_frequency(0, sidetone_frequency);

        let arc_tone_generator = Arc::new(Mutex::new(tone_generator));
        let fixture_arc_tone_generator = arc_tone_generator.clone();
        let playback = Playback::new(terminate.clone(), scheduled_thread_pool, arc_tone_generator, keying_event_tone_channel_tx.clone());
        let fixture = PlaybackFixture {
            terminate,
            scheduled_thread_pool: fixture_scheduled_thread_pool,
            keying_event_tx: Arc::new(Mutex::new(keying_event_tx)),
            source_encoder_rx,
            _source_encoder: source_encoder,
            source_decoder: source_decoder,
            tone_generator: fixture_arc_tone_generator,
            pa: Arc::new(pa::PortAudio::new().unwrap()),
            playback,
        };
        let output_settings = open_output_audio_device(&fixture.pa, dev).unwrap();
        info!("Initialising audio callback...");
        fixture.tone_generator.lock().unwrap().start_callback(&fixture.pa, output_settings).unwrap();

        info!("Fixture setup sleeping");
        test_util::wait_n_ms(100); // give things time to start
        info!("Fixture setup out of sleep");

        fixture
    }

    impl Drop for PlaybackFixture {
        fn drop(&mut self) {
            debug!("PlaybackFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("PlaybackFixture ...set terminate flag");
        }
    }

    const CALLSIGN_HASH: u16 = 0x1234u16;
    const AUDIO_OFFSET: u16 = 700;

    #[rstest]
    #[serial]
    #[ignore]
    pub fn playback_cq_cq(mut fixture: PlaybackFixture) {
        send_keying("cq-cq-keying.csv", &fixture);

        info!("Waiting for source encoder data");
        loop {
            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(4)) {
                Ok(encoding) => {
                    let decoded = fixture.source_decoder.source_decode(encoding.block);
                    info!("Playing back frame");
                    fixture.playback.play(decoded, CALLSIGN_HASH, AUDIO_OFFSET);
                }
                Err(_) => {
                    info!("No more source encoder data - exiting loop");
                    break;
                }
            }
        }
        info!("Waiting for playback to end...");
        test_util::wait_n_ms(4000);
        info!("End of test")
    }

    struct KeyingPlayback {
        item: KeyingEvent,
        output_tx: Arc<Mutex<Bus<KeyingEvent>>>,
    }

    impl Task for KeyingPlayback {
        fn run(self) {
            let mut output = self.output_tx.lock().unwrap();
            output.broadcast(self.item);
        }
    }

    fn send_keying(keying_filename: &str, fixture: &PlaybackFixture) {
        match ReaderBuilder::default().has_headers(false).from_path(keying_filename) {
            Ok(mut rtr) => {
                let mut row = StringRecord::new();
                let mut keying_offset = 0;
                //info!("Keying start");
                let start_task = KeyingPlayback { item: KeyingEvent::Start(), output_tx: fixture.keying_event_tx.clone() };
                fixture.scheduled_thread_pool.schedule_ms(keying_offset, start_task);
                while rtr.read_record(&mut row).is_ok() {
                    match  row.get(0) {
                        None => {
                            break;
                        }
                        Some(mark_space) => {
                            let duration = row.get(1).unwrap().parse::<u32>().unwrap();
                            //info!("Keying input: {} {}", mark_space, duration);
                            // Each row is the end of a keying
                            keying_offset += duration;
                            let timed_task = KeyingPlayback { item: KeyingEvent::Timed(KeyingTimedEvent { up: mark_space.eq("MARK"), duration: duration as KeyerEdgeDurationMs }), output_tx: fixture.keying_event_tx.clone() };
                            fixture.scheduled_thread_pool.schedule_ms(keying_offset, timed_task);
                        }
                    };
                }
                keying_offset += 250; // a small break-in delay
                //info!("Keying end");
                let end_task = KeyingPlayback { item: KeyingEvent::End(), output_tx: fixture.keying_event_tx.clone() };
                fixture.scheduled_thread_pool.schedule_ms(keying_offset, end_task);
            }
            Err(err) => { panic!("Can't read CSV file: {}", err); }
        };
    }
}

