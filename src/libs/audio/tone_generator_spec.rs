extern crate hamcrest2;
extern crate portaudio;

#[cfg(test)]
mod tone_generator_spec {
    use bus::Bus;
    use log::{debug, info};
    use std::env;
    use rstest::*;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use crate::libs::audio::audio_devices::open_output_audio_device;
    use crate::libs::audio::tone_generator::ToneGenerator;
    use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyingTimedEvent};
    use crate::libs::util::test_util;
    use portaudio as pa;
    use portaudio::PortAudio;

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
        tone_generator: ToneGenerator,
        pa: Arc<PortAudio>,
    }

    #[fixture]
    fn fixture() -> ToneGeneratorFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        let fixture_keying_event_tx = Arc::new(Mutex::new(keying_event_tx));

        info!("Initialising audio callback...");
        let dev = "Built-in Output";
        let sidetone_frequency = 600 as u16;
        let tone_generator = ToneGenerator::new(sidetone_frequency,
                                                    keying_event_rx, terminate.clone());
        let mut fixture = ToneGeneratorFixture {
            terminate,
            keying_event_tx: fixture_keying_event_tx,
            tone_generator,
            pa: Arc::new(pa::PortAudio::new().unwrap()),
        };
        let output_settings = open_output_audio_device(&fixture.pa, dev).unwrap();
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

    #[rstest]
    #[serial]
    pub fn play_paris_at_12wpm(fixture: ToneGeneratorFixture) {
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
        play_in_real_time(paris_keying, &fixture.keying_event_tx);
    }


    fn play_in_real_time(keying: Vec<KeyingEvent>, keying_bus_tx: &Arc<Mutex<Bus<KeyingEvent>>>) {
        debug!("Playing keying sequence...");
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
        }
        debug!("Finished playing keying sequence");
    }
}
