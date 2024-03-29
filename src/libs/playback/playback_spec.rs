extern crate hamcrest2;

#[cfg(test)]
mod playback_spec {
    use std::env;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};

    use bus::Bus;
    use hamcrest2::prelude::*;
    use log::{debug, info};
    use portaudio as pa;
    use portaudio::PortAudio;
    use rstest::*;
    use crate::libs::application::application::{BusInput, BusOutput};
    use crate::libs::audio::audio_devices::open_output_audio_device;
    use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneGenerator};
    use crate::libs::config_dir::config_dir;
    use crate::libs::config_file::config_file::ConfigurationStore;
    use crate::libs::playback::playback::Playback;
    use crate::libs::source_codec::source_encoding::Frame;
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct PlaybackFixture {
        terminate: Arc<AtomicBool>,
        tone_generator: Arc<Mutex<ToneGenerator>>,
        pa: Arc<PortAudio>,
        playback: Playback,
    }

    #[fixture]
    fn fixture() -> PlaybackFixture {
        info!("starting fixture");
        let home_dir = dirs::home_dir();
        let config_path = config_dir::configuration_directory(home_dir).unwrap();
        let config = ConfigurationStore::new(config_path).unwrap();

        let terminate = Arc::new(AtomicBool::new(false));
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

        let keying_event_tone_channel_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>> = Arc::new(Mutex::new(Bus::new(16)));
        let keying_event_tone_channel_rx = keying_event_tone_channel_tx.lock().unwrap().add_rx();

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
        let mut playback = Playback::new(terminate.clone(), scheduled_thread_pool, arc_tone_generator);
        playback.set_output_tx(keying_event_tone_channel_tx.clone());

        let fixture = PlaybackFixture {
            terminate,
            tone_generator: fixture_arc_tone_generator,
            pa: Arc::new(pa::PortAudio::new().unwrap()),
            playback,
        };
        let output_settings = open_output_audio_device(&fixture.pa, config.get_audio_out_device().as_str()).unwrap();
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
    pub fn playback_one_user_two_frames_perfects(mut fixture: PlaybackFixture) {
        info!("Sending a message in...");
        let first_cq_frame = vec![
            Frame::WPMPolarity { wpm: 20, polarity: true }, // 60 / 180 / 420
            Frame::KeyingPerfectDah, // - 180
            Frame::KeyingPerfectDit, //    60
            Frame::KeyingPerfectDit, // .  60
            Frame::KeyingPerfectDit, //    60
            Frame::KeyingPerfectDah, // - 180
            Frame::KeyingPerfectDit, //    60
            Frame::KeyingPerfectDit, // .  60
            Frame::KeyingPerfectDah, // chargap 180
            Frame::KeyingPerfectDah, // - 180
            Frame::KeyingPerfectDit, //    60
            Frame::KeyingPerfectDah, // - 180
            Frame::KeyingPerfectDit, //    60
            Frame::KeyingPerfectDit, // .  60
        ];                           // =1380

        // There's no last playback schedule time until play has been given some Frames..
        assert_that!(fixture.playback.get_last_playback_schedule_time(CALLSIGN_HASH, AUDIO_OFFSET), none());

        fixture.playback.play(Ok(first_cq_frame), CALLSIGN_HASH, AUDIO_OFFSET);

        let maybe_last_schedule_time = fixture.playback.get_last_playback_schedule_time(CALLSIGN_HASH, AUDIO_OFFSET);
        if let Some(last_schedule_time) = maybe_last_schedule_time {
            assert_that!( last_schedule_time, equal_to(2379));
        } else {
            panic!("Should have stored station details");
        }
        let second_cq_frame = vec![
            Frame::WPMPolarity { wpm: 20, polarity: false },
            Frame::KeyingPerfectDit,
            Frame::KeyingPerfectDah, // -
            Frame::KeyingEnd,
        ];
        fixture.playback.play(Ok(second_cq_frame), CALLSIGN_HASH, AUDIO_OFFSET);
        info!("Waiting for playback to end...");
        test_util::wait_n_ms(3000);
        info!("End of test")
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn playback_deltas(mut fixture: PlaybackFixture) {
        info!("Sending a message in...");
        let first_cq_frame = vec![
            Frame::WPMPolarity { wpm: 20, polarity: true }, // 60 / 180 / 420
            Frame::KeyingDeltaDah { delta: 1 }, // - 180
            Frame::KeyingDeltaDit { delta: -1 }, //    60
            Frame::KeyingDeltaDit { delta: -1 }, // .  60
            Frame::KeyingDeltaDit { delta: 1 }, //    60
            Frame::KeyingDeltaDah { delta: 1 }, // - 180
            Frame::KeyingDeltaDit { delta: -1 }, //    60
            Frame::KeyingDeltaDit { delta: -1 }, // .  60
            Frame::KeyingDeltaDah { delta: 1 }, // chargap 180
            Frame::KeyingDeltaDah { delta: 1 }, // - 180
            Frame::KeyingDeltaDit { delta: -1 }, //    60
            Frame::KeyingDeltaDah { delta: -1 }, // - 180
            Frame::KeyingDeltaDit { delta: 1 }, //    60
            Frame::KeyingDeltaDit { delta: 1 }, // .  60
        ];                           // =1380
        fixture.playback.play(Ok(first_cq_frame), CALLSIGN_HASH, AUDIO_OFFSET);

        let second_cq_frame = vec![
            Frame::WPMPolarity { wpm: 20, polarity: false },
            Frame::KeyingDeltaDit { delta: -1 },
            Frame::KeyingDeltaDah { delta: -1 }, // -
            Frame::KeyingEnd,
        ];
        fixture.playback.play(Ok(second_cq_frame), CALLSIGN_HASH, AUDIO_OFFSET);

        info!("Waiting for playback to end...");
        test_util::wait_n_ms(3000);
        info!("End of test")
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn playback_naives(mut fixture: PlaybackFixture) {
        info!("Sending a message in...");
        let first_cq_frame = vec![
            Frame::WPMPolarity { wpm: 20, polarity: true }, // 60 / 180 / 420
            Frame::KeyingNaive { duration: 180 }, // - 180
            Frame::KeyingNaive { duration: 60 }, //    60
            Frame::KeyingNaive { duration: 60 }, // .  60
            Frame::KeyingNaive { duration: 60 }, //    60
            Frame::KeyingNaive { duration: 180 }, // - 180
            Frame::KeyingNaive { duration: 60 }, //    60
            Frame::KeyingNaive { duration: 60 }, // .  60
            Frame::KeyingNaive { duration: 180 }, // chargap 180
            Frame::KeyingNaive { duration: 180 }, // - 180
            Frame::KeyingNaive { duration: 60 }, //    60
            Frame::KeyingNaive { duration: 180 }, // - 180
            Frame::KeyingNaive { duration: 60 }, //    60
            Frame::KeyingNaive { duration: 60 }, // .  60
        ];                           // =1380
        fixture.playback.play(Ok(first_cq_frame), CALLSIGN_HASH, AUDIO_OFFSET);

        let second_cq_frame = vec![
            Frame::WPMPolarity { wpm: 20, polarity: false },
            Frame::KeyingNaive { duration: 60 },
            Frame::KeyingNaive { duration: 180 }, // -
            Frame::KeyingEnd,
        ];
        fixture.playback.play(Ok(second_cq_frame), CALLSIGN_HASH, AUDIO_OFFSET);

        info!("Waiting for playback to end...");
        test_util::wait_n_ms(3000);
        info!("End of test")
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn playback_allocates_tone_generator_channels(mut fixture: PlaybackFixture) {
        assert_eq!(fixture.tone_generator.lock().unwrap().test_get_enabled_states(), vec![true]);
        let frame = vec![
            Frame::WPMPolarity { wpm: 20, polarity: true }, // 60 / 180 / 420
            Frame::KeyingNaive { duration: 180 }, // - 180
            Frame::KeyingNaive { duration: 60 }, //    60
            Frame::KeyingEnd,
        ];                           // =1380
        fixture.playback.play(Ok(frame), CALLSIGN_HASH, AUDIO_OFFSET);
        assert_eq!(fixture.tone_generator.lock().unwrap().test_get_enabled_states(), vec![true, true]);

        info!("Waiting for playback to end...");
        test_util::wait_n_ms(200);
        info!("End of test")
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn playback_deallocates_tone_generator_channels(mut fixture: PlaybackFixture) {
        let frame = vec![
            Frame::WPMPolarity { wpm: 20, polarity: true }, // 60 / 180 / 420
            Frame::KeyingNaive { duration: 180 }, // - 180
            Frame::KeyingNaive { duration: 60 }, //    60
            Frame::KeyingEnd,
        ];                           // =1380
        fixture.playback.play(Ok(frame), CALLSIGN_HASH, AUDIO_OFFSET);
        assert_eq!(fixture.tone_generator.lock().unwrap().test_get_enabled_states(), vec![true, true]);

        info!("Waiting for expiry...");
        test_util::wait_n_ms(21000);
        info!("Expiring...");
        fixture.playback.expire(); // called by play, in real life - invoke directly in test.
        assert_eq!(fixture.tone_generator.lock().unwrap().test_get_enabled_states(), vec![true]);

        info!("End of test")
    }

}

