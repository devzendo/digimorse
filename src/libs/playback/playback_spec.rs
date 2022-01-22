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
    use crate::libs::audio::audio_devices::open_output_audio_device;
    use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneGenerator};
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
        let terminate = Arc::new(AtomicBool::new(false));
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

        let keying_event_tone_channel_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>> = Arc::new(Mutex::new(Bus::new(16)));
        let keying_event_tone_channel_rx = keying_event_tone_channel_tx.lock().unwrap().add_rx();

        let dev = "Built-in Output";
        let sidetone_frequency = 600 as u16;
        info!("Instantiating tone generator...");
        let mut tone_generator = ToneGenerator::new(sidetone_frequency,
                                                    keying_event_tone_channel_rx, terminate.clone());
        info!("Setting audio freqency...");
        tone_generator.set_audio_frequency(0, sidetone_frequency);

        let arc_tone_generator = Arc::new(Mutex::new(tone_generator));
        let fixture_arc_tone_generator = arc_tone_generator.clone();
        let playback = Playback::new(terminate.clone(), scheduled_thread_pool, arc_tone_generator, keying_event_tone_channel_tx.clone());
        let fixture = PlaybackFixture {
            terminate,
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
    pub fn playback_one_user(mut fixture: PlaybackFixture) {
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
            assert_that!( last_schedule_time, equal_to(1380));
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
        test_util::wait_n_ms(2500);
        info!("End of test")
    }
}

