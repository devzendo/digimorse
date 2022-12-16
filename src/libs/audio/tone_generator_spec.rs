extern crate hamcrest2;
extern crate portaudio;

// These are all manually run (and aurally asserted correct).
#[cfg(test)]
mod tone_generator_spec {
    use std::collections::BTreeMap;
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
    use crate::libs::keyer_io::keyer_io::KeyingEvent;
    use crate::libs::transform_bus::transform_bus::TransformBus;
    use crate::libs::util::test_util;
    use portaudio::PortAudio;
    use crate::libs::application::application::{BusInput, BusOutput};
    use crate::libs::conversion::conversion::text_to_keying;
    use crate::libs::conversion::paris::PARIS_KEYING_12WPM;

    const TABLE_SIZE: usize = 256;

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
        keying_event_tone_channel_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>>,
        // Not read, but needs storing to maintain lifetime
        _transform_bus: Arc<Mutex<TransformBus<KeyingEvent, KeyingEventToneChannel>>>,
        tone_generator: ToneGenerator,
        pa: Arc<PortAudio>,
        paris_keying_12_wpm: Vec<KeyingEvent>,
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

        let keying_event_tone_channel_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>> = Arc::new(Mutex::new(Bus::new(16)));
        let fixture_keying_event_tone_channel_tx = keying_event_tone_channel_tx.clone();
        let mut transform_bus = TransformBus::new(add_sidetone_channel_to_keying_event, terminate.clone());
        transform_bus.set_input_rx(Arc::new(Mutex::new(keying_event_rx)));
        transform_bus.set_output_tx(keying_event_tone_channel_tx);
        let arc_transform_bus = Arc::new(Mutex::new(transform_bus));
        let keying_event_tone_channel_rx = arc_transform_bus.lock().unwrap().add_reader();

        let old_macbook = true;
        let dev = if old_macbook {"Built-in Output"} else {"MacBook Pro Speakers"};
        let sidetone_frequency = 600 as u16;
        info!("Instantiating tone generator...");
        let tone_generator_keying_event_tone_channel_rx = Arc::new(Mutex::new(keying_event_tone_channel_rx));
        let mut tone_generator = ToneGenerator::new(sidetone_frequency,
                                                    terminate.clone());
        tone_generator.set_input_rx(tone_generator_keying_event_tone_channel_rx);

        info!("Setting audio freqency...");
        tone_generator.set_audio_frequency(0, sidetone_frequency);

        let mut fixture = ToneGeneratorFixture {
            terminate,
            keying_event_tx: fixture_keying_event_tx,
            keying_event_tone_channel_tx: fixture_keying_event_tone_channel_tx,
            _transform_bus: arc_transform_bus,
            tone_generator,
            pa: Arc::new(PortAudio::new().unwrap()),
            paris_keying_12_wpm: PARIS_KEYING_12WPM.to_vec(),
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
            // debug!("sine[{}] = {}", i, sine[i]);
            print!("{}_f32, ", sine[i]);
            if sine[i] > max_sine {
                max_sine = sine[i];
            }
            if sine[i] < min_sine {
                min_sine = sine[i];
            }
        }
        println!();
        debug!("min {} max {}", min_sine, max_sine);
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn play_paris_at_12wpm_as_sidetone(mut fixture: ToneGeneratorFixture) {
        play_keying_events_in_real_time(fixture.paris_keying_12_wpm.clone(), &fixture.keying_event_tx, &mut fixture.tone_generator, 0);
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn play_keying_as_sidetone(mut fixture: ToneGeneratorFixture) {
        let keying = text_to_keying(20, "CQ CQ DX DE M0CUV CQ CQ PSE K");
        play_keying_events_in_real_time(keying, &fixture.keying_event_tx, &mut fixture.tone_generator, 0);
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn play_keying_as_sidetone_increasing_frequency(mut fixture: ToneGeneratorFixture) {
        let keying = text_to_keying(20, "CQ CQ CQ CQ DE M0CUV M0CUV PSE K");
        play_keying_events_in_real_time(keying, &fixture.keying_event_tx, &mut fixture.tone_generator, 2);
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn play_single_keying_to_channel_with_merge(mut fixture: ToneGeneratorFixture) {
        let a_keying = text_to_keying(40, "CQ CQ CQ CQ DE M0CUV M0CUV PSE K");
        let a_channel = fixture.tone_generator.allocate_channel(600);
        assert_that!(a_channel, equal_to(1));
        let a_keying_tones = a_keying.iter().map(|k| KeyingEventToneChannel{ keying_event: k.clone(), tone_channel: a_channel }).collect();
        let mut merged = KeyingToneMerger::new();
        merged.add(3000, a_keying_tones);
        let merged = merged.merge();
        play_keying_events_with_tone_channel_in_real_time(merged, &fixture.keying_event_tone_channel_tx);
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn play_multiple_keyings_to_channels_with_merge(mut fixture: ToneGeneratorFixture) {
        let a_keying = text_to_keying(20, "CQ CQ CQ CQ DE M0CUV M0CUV PSE K");
        let b_keying = text_to_keying(12, "CQ TEST UR 599 QRZ?");
        let c_keying = text_to_keying(35, "N9XYZ DE M0CUV = MNI TNX FER CALL = UR RST 489 489 = SO HW CPY? = N9XYZ DE M0CUV KN");
        let a_channel = fixture.tone_generator.allocate_channel(600);
        assert_that!(a_channel, equal_to(1));
        let b_channel = fixture.tone_generator.allocate_channel(800);
        assert_that!(b_channel, equal_to(2));
        let c_channel = fixture.tone_generator.allocate_channel(400);
        assert_that!(c_channel, equal_to(3));
        let a_keying_tones = a_keying.iter().map(|k| KeyingEventToneChannel{ keying_event: k.clone(), tone_channel: a_channel }).collect();
        let b_keying_tones = b_keying.iter().map(|k| KeyingEventToneChannel{ keying_event: k.clone(), tone_channel: b_channel }).collect();
        let c_keying_tones = c_keying.iter().map(|k| KeyingEventToneChannel{ keying_event: k.clone(), tone_channel: c_channel }).collect();
        let mut merged = KeyingToneMerger::new();
        merged.add(3000, a_keying_tones);
        merged.add(5000, b_keying_tones);
        merged.add(100, c_keying_tones);
        let merged = merged.merge();
        play_keying_events_with_tone_channel_in_real_time(merged, &fixture.keying_event_tone_channel_tx);
    }

    // Utility object for merging multiple KeyingEventToneChannel streams --------------------------

    pub type KeyerOffsetFromStartMs = u32;

    struct KeyingToneMerger {
        timing_map: BTreeMap<KeyerOffsetFromStartMs, Vec<KeyingEventToneChannel>>,
    }

    impl KeyingToneMerger {
        pub fn new() -> Self {
            Self {
                timing_map: BTreeMap::new(),
            }
        }

        pub fn add(&mut self, delay_ms: KeyerOffsetFromStartMs, keying_event_with_tones: Vec<KeyingEventToneChannel>) {
            let mut time = delay_ms;
            for kevt in keying_event_with_tones {
                match kevt.keying_event.clone() {
                    KeyingEvent::Timed(timed) => {
                        time += timed.duration as u32;
                    }
                    KeyingEvent::Start() => {}
                    KeyingEvent::End() => {}
                }
                self.timing_map.entry(time).or_insert(Vec::new()).push(kevt.clone());
            }
        }

        pub fn merge(&mut self) -> Vec<(KeyerOffsetFromStartMs, Vec<KeyingEventToneChannel>)> {
            let mut out: Vec<(KeyerOffsetFromStartMs, Vec<KeyingEventToneChannel>)> = Vec::new();
            for (time, vec_kevt) in &self.timing_map {
                out.push((*time, (*vec_kevt).clone()));
            }
            out
        }
    }



    fn play_keying_events_in_real_time(keying: Vec<KeyingEvent>, keying_bus_tx: &Arc<Mutex<Bus<KeyingEvent>>>, tone_generator: &mut ToneGenerator, freq_increase: u16) {
        debug!("Playing keying sequence...");
        let mut freq = 600;
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
            if freq_increase != 0 {
                tone_generator.set_audio_frequency(0, freq);
                freq += freq_increase;
                //eprintln!("Frequency: {} Hz", freq); // 740 Hz is particularly bad for distorted tone
            }
        }
        debug!("Finished playing keying sequence");
    }

    fn play_keying_events_with_tone_channel_in_real_time(timed_ketcs: Vec<(KeyerOffsetFromStartMs, Vec<KeyingEventToneChannel>)>, keying_bus_tx: &Arc<Mutex<Bus<KeyingEventToneChannel>>>) {
        debug!("Playing keying sequence...");
        let mut time = 0;
        for timed_ketc in timed_ketcs {
            let (keying_time, these_keyings) = (timed_ketc.0, timed_ketc.1);
            debug!("Time is {}, Keying time is {}, Keyings for now: {:?}", time, keying_time, these_keyings);
            let delay = keying_time - time;
            if delay > 0 {
                debug!("Delaying for {}ms", delay);
                spin_sleep::sleep(Duration::from_millis(delay as u64));
                time += delay;
            }
            for this_keying in these_keyings {
                debug!("Broadcasting {}", this_keying);
                keying_bus_tx.lock().unwrap().broadcast(this_keying);
            }
        }
        debug!("Finished playing keying sequence");
    }
}
