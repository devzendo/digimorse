extern crate hamcrest2;

#[cfg(test)]
mod tone_generator_channel_alloc_spec {
    use bus::Bus;
    use log::{debug, info};
    use std::env;
    use rstest::*;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use crate::libs::application::application::BusInput;
    use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneGenerator};
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct ToneGeneratorFixture {
        terminate: Arc<AtomicBool>,
        _keying_event_tone_channel_tx: Bus<KeyingEventToneChannel>,
        tone_generator: ToneGenerator,
    }

    #[fixture]
    fn fixture() -> ToneGeneratorFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tone_channel_tx = Bus::new(16);
        let keying_event_tone_channel_rx = keying_event_tone_channel_tx.add_rx();

        let sidetone_frequency = 600 as u16;
        info!("Instantiating tone generator...");
        let tone_generator_keying_event_tone_channel_rx = Arc::new(Mutex::new(keying_event_tone_channel_rx));
        let mut tone_generator = ToneGenerator::new(sidetone_frequency,
                                                    terminate.clone());
        tone_generator.set_input_rx(tone_generator_keying_event_tone_channel_rx);

        let fixture = ToneGeneratorFixture {
            terminate,
            _keying_event_tone_channel_tx: keying_event_tone_channel_tx,
            tone_generator,
        };

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
    pub fn initial_conditions(mut fixture: ToneGeneratorFixture) {
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true]);
    }

    #[rstest]
    #[serial]
    pub fn allocate_allocates(mut fixture: ToneGeneratorFixture) {
        fixture.tone_generator.allocate_channel(800);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, true]);
    }

    #[rstest]
    #[serial]
    pub fn deallocate_zero_does_not_disable(mut fixture: ToneGeneratorFixture) {
        fixture.tone_generator.deallocate_channel(0); // does not
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true]);
    }

    #[rstest]
    #[serial]
    pub fn deallocate_nonzero_disables(mut fixture: ToneGeneratorFixture) {
        fixture.tone_generator.allocate_channel(800);
        fixture.tone_generator.allocate_channel(1000);
        fixture.tone_generator.deallocate_channel(1);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, false, true]);
    }

    #[rstest]
    #[serial]
    pub fn deallocate_end_truncates(mut fixture: ToneGeneratorFixture) {
        fixture.tone_generator.allocate_channel(1000); // if we disable the last, the array should be truncated
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, true]);
        fixture.tone_generator.deallocate_channel(1);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true]);
    }

    #[rstest]
    #[serial]
    pub fn deallocate_past_end(mut fixture: ToneGeneratorFixture) {
        fixture.tone_generator.allocate_channel(1000); // if we disable the last, the array should be truncated
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, true]);
        fixture.tone_generator.deallocate_channel(2); // does nothing
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, true]);
    }

    #[rstest]
    #[serial]
    pub fn deallocate_end_truncates_all_disabled(mut fixture: ToneGeneratorFixture) {
        fixture.tone_generator.allocate_channel(800);
        fixture.tone_generator.allocate_channel(900);
        fixture.tone_generator.allocate_channel(950);
        fixture.tone_generator.allocate_channel(1000); // if we disable the last, the array should be truncated
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, true, true, true, true]);
        fixture.tone_generator.deallocate_channel(1);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, false, true, true, true]);
        fixture.tone_generator.deallocate_channel(2);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, false, false, true, true]);
        fixture.tone_generator.deallocate_channel(3);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, false, false, false, true]);
        fixture.tone_generator.deallocate_channel(4);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true]);
    }

    #[rstest]
    #[serial]
    pub fn allocate_allocates_first_disabled(mut fixture: ToneGeneratorFixture) {
        fixture.tone_generator.allocate_channel(800);
        fixture.tone_generator.allocate_channel(900);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, true, true]);
        fixture.tone_generator.deallocate_channel(1);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, false, true]);
        fixture.tone_generator.allocate_channel(1000);
        assert_eq!(fixture.tone_generator.test_get_enabled_states(), vec![true, true, true]);
    }
}