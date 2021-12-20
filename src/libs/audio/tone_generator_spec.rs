extern crate hamcrest2;
extern crate portaudio;

#[cfg(test)]
mod tone_generator_spec {
    use bus::Bus;
    use log::{debug, info};
    use std::env;
    use rstest::*;
    use std::f64::consts::PI;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use hamcrest2::prelude::*;
    use crate::libs::audio::audio_devices::open_output_audio_device;
    use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneGenerator};
    use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyingTimedEvent};
    use crate::libs::transform_bus::transform_bus::TransformBus;
    use crate::libs::util::test_util;
    use portaudio as pa;
    use portaudio::PortAudio;


    const TABLE_SIZE: usize = 200;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct ToneGeneratorFixture {
        terminate: Arc<AtomicBool>,
        keying_event_tx: Arc<Mutex<Bus<KeyingEvent>>>,
        // Not read, but needs storing to maintain lifetime
        _transform_bus: Arc<Mutex<TransformBus<KeyingEvent, KeyingEventToneChannel>>>,
        tone_generator: ToneGenerator,
        pa: Arc<PortAudio>,
    }

    fn add_sidetone_channel_to_keying_event(keying_event: KeyingEvent) -> KeyingEventToneChannel {
        return KeyingEventToneChannel { keying_event, tone_channel: 0 };
    }

    #[fixture]
    fn fixture() -> ToneGeneratorFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        let fixture_keying_event_tx = Arc::new(Mutex::new(keying_event_tx));

        let mut keying_event_tone_channel_tx: Bus<KeyingEventToneChannel> = Bus::new(16);
        let transform_bus = TransformBus::new(keying_event_rx, keying_event_tone_channel_tx, add_sidetone_channel_to_keying_event, terminate.clone());
        let arc_transform_bus = Arc::new(Mutex::new(transform_bus));
        let keying_event_tone_channel_rx = arc_transform_bus.lock().unwrap().add_reader();

        let dev = "Built-in Output";
        let sidetone_frequency = 600 as u16;
        info!("Instantiating tone generator...");
        let mut tone_generator = ToneGenerator::new(sidetone_frequency,
                                                    keying_event_tone_channel_rx, terminate.clone());
        info!("Setting audio freqency...");
        tone_generator.set_audio_frequency(0, 600);
        let mut fixture = ToneGeneratorFixture {
            terminate,
            keying_event_tx: fixture_keying_event_tx,
            _transform_bus: arc_transform_bus,
            tone_generator,
            pa: Arc::new(pa::PortAudio::new().unwrap()),
        };
        let output_settings = open_output_audio_device(&fixture.pa, dev).unwrap();
        info!("Initialising audio callback...");
        fixture.tone_generator.start_callback(&fixture.pa, output_settings).unwrap();

        info!("Fixture setup sleeping");
        test_util::wait_n_ms(100); // give things time to start
        info!("Fixture setup out of sleep");

        fixture
    }

    impl Drop for ToneGeneratorFixture {
        fn drop(&mut self) {
            debug!("SourceEncoderFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_n_ms(100);
            debug!("SourceEncoderFixture ...set terminate flag");
        }
    }

    #[test]
    fn sines() {
        let mut min_sine = 0.0;
        let mut max_sine = 0.0;
        let mut sine: [f32; TABLE_SIZE] = [0.0; TABLE_SIZE];
        for i in 0..TABLE_SIZE {
            sine[i] = (i as f64 / TABLE_SIZE as f64 * PI * 2.0).sin() as f32;
            debug!("sine[{}] = {}", i, sine[i]);
            if sine[i] > max_sine {
                max_sine = sine[i];
            }
            if sine[i] < min_sine {
                min_sine = sine[i];
            }
        }
        debug!("min {} max {}", min_sine, max_sine);
    }

    #[rstest]
    #[serial]
    pub fn play_paris_at_12wpm(mut fixture: ToneGeneratorFixture) {
        let paris_keying = vec![
            KeyingEvent::Start(),

            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),

            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 300 }),

            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),

            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 300 }),

            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),

            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 300 }),

            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),

            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 300 }),

            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),

            KeyingEvent::End(),
        ];
        play_in_real_time(paris_keying, &fixture.keying_event_tx, &mut fixture.tone_generator);
    }


    #[rstest]
    #[serial]
    pub fn play_multiple_keyings(mut fixture: ToneGeneratorFixture) {
        let a_keying = text_to_keying(20, "CQ CQ CQ CQ DE M0CUV M0CUV PSE K");
        let b_keying = text_to_keying(12, "CQ TEST UR 599 QRZ?");
        let c_keying = text_to_keying(16, "N9XYZ DE M0CUV = MNI TNX FER CALL = UR RST 489 489 = SO HW CPY? = N9XYZ DE M0CUV KN");
        let a_channel = fixture.tone_generator.allocate_channel(600);
        assert_that!(a_channel, equal_to(1));
        let b_channel = fixture.tone_generator.allocate_channel(800);
        assert_that!(b_channel, equal_to(2));
        let c_channel = fixture.tone_generator.allocate_channel(400);
        assert_that!(c_channel, equal_to(3));
        let a_keying_tones = a_keying.iter().map(|k| KeyingEventToneChannel{ keying_event: k.clone(), tone_channel: a_channel }).collect();
        let b_keying_tones = b_keying.iter().map(|k| KeyingEventToneChannel{ keying_event: k.clone(), tone_channel: b_channel }).collect();
        let c_keying_tones = c_keying.iter().map(|k| KeyingEventToneChannel{ keying_event: k.clone(), tone_channel: c_channel }).collect();
        let mut interspersed = KeyingToneMerger::new();
        interspersed.add(3000, a_keying_tones);
        interspersed.add(5000, b_keying_tones);
        interspersed.add(100, c_keying_tones);
        let merged = interspersed.merge();
        // TODO put merged into the keying_with_tone_channel bus, with delays.
    }

    fn text_to_keying(wpm: u32, text: &str) -> Vec<KeyingEvent> {
        vec![]
    }

    struct KeyingToneMerger {

    }

    impl KeyingToneMerger {
        pub fn new() -> Self {
            Self {

            }
        }

        pub fn add(&mut self, delay_ms: u16, keying_event_with_tones: Vec<KeyingEventToneChannel>) {

        }

        pub fn merge(&mut self) -> Vec<KeyingEventToneChannel> {
            vec![]
        }
    }

    fn play_in_real_time(keying: Vec<KeyingEvent>, keying_bus_tx: &Arc<Mutex<Bus<KeyingEvent>>>, tone_generator: &mut ToneGenerator) {
        debug!("Playing keying sequence...");
        let mut freq = 400;
        for k in keying {
            let timed_k = k.clone();
            match k {
                KeyingEvent::Start() | KeyingEvent::End() => {
                    keying_bus_tx.lock().unwrap().broadcast(k);
                }
                KeyingEvent::Timed(timed) => {
                    spin_sleep::sleep(Duration::from_millis(timed.duration as u64));
                    keying_bus_tx.lock().unwrap().broadcast(timed_k);
                }
            }
            tone_generator.set_audio_frequency(0, freq);
            freq += 1;
        }
        debug!("Finished playing keying sequence");
    }
}
