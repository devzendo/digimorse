extern crate hamcrest2;

// These are all manually run (and asserted correct aurally and with Audio Hijack / spectrum analyser).
#[cfg(test)]
mod transmitter_spec {
    use std::env;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};

    use bus::Bus;
    use hamcrest2::prelude::*;
    use log::{debug, info};
    use portaudio::PortAudio;
    use rstest::*;

    use crate::libs::application::application::BusInput;
    use crate::libs::audio::audio_devices::open_output_audio_device;
    use crate::libs::channel_codec::channel_encoding::ChannelEncoding;
    use crate::libs::channel_codec::sample_channel_encoding::sample_channel_encoding;
    use crate::libs::test::test_hardware;
    use crate::libs::transmitter::transmitter::{AmplitudeMax, AudioFrequencyHz, maximum_number_of_symbols, Transmitter};
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct TransmitterFixture {
        terminate: Arc<AtomicBool>,
        channel_encoding_tx: Arc<Mutex<Bus<ChannelEncoding>>>,
        transmitter: Transmitter,
        pa: Arc<PortAudio>,
    }

    #[test]
    fn test_maximum_number_of_symbols() {
        // If the size of the source encoder's output changes, this will need to be recalculated.
        assert_eq!(maximum_number_of_symbols(), 71);
        // Note this does not count ramp up/down, they're not full symbols.
    }

    #[fixture]
    fn fixture() -> TransmitterFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut channel_encoding_tx = Bus::new(16);
        let channel_encoding_rx = channel_encoding_tx.add_rx();
        let fixture_channel_encoding_tx = Arc::new(Mutex::new(channel_encoding_tx));

        let audio_frequency = 600 as AudioFrequencyHz;
        info!("Instantiating transmitter...");
        let transmitter_channel_encoding_rx = Arc::new(Mutex::new(channel_encoding_rx));
        let mut transmitter = Transmitter::new(audio_frequency,
                                                 terminate.clone());
        transmitter.set_input_rx(transmitter_channel_encoding_rx);

        let mut fixture = TransmitterFixture {
            terminate,
            channel_encoding_tx: fixture_channel_encoding_tx,
            transmitter,
            pa: Arc::new(PortAudio::new().unwrap()),
        };
        let speaker = test_hardware::get_current_system_speaker_name();
        info!("Output to speaker '{}'", speaker);
        let output_settings = open_output_audio_device(&fixture.pa, speaker.as_str()).unwrap();
        info!("Setting amplitude max");
        fixture.transmitter.set_amplitude_max(1.0 as AmplitudeMax);
        info!("Initialising audio callback...");
        fixture.transmitter.start_callback(&fixture.pa, output_settings).unwrap();
        info!("Setting audio frequency...");
        fixture.transmitter.set_audio_frequency_allocate_buffer(audio_frequency);

        info!("Fixture setup sleeping");
        test_util::wait_n_ms(100); // give things time to start
        info!("Fixture setup out of sleep");

        fixture
    }

    impl Drop for TransmitterFixture {
        fn drop(&mut self) {
            debug!("TransmitterFixture about to set terminate flag...");
            test_util::wait_n_ms(1000); // to detect any clunk when closing streams?
            debug!("TransmitterFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("TransmitterFixture ...set terminate flag");
        }
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn transmitter_is_silent_when_nothing_to_play(fixture: TransmitterFixture) {
        assert_that!(fixture.transmitter.is_silent(), equal_to(true));
        test_util::wait_5_ms();
        assert_that!(fixture.transmitter.is_silent(), equal_to(true));
        test_util::wait_5_ms();
        assert_that!(fixture.transmitter.is_silent(), equal_to(true));
        debug!("Done!");
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn play_sample_gfsk_encoded_channel_encoding(fixture: TransmitterFixture) {
        let channel_encoding = sample_channel_encoding();
        play_encoding(fixture,  channel_encoding);
    }

    pub fn rising_channel_encoding() -> ChannelEncoding {
        ChannelEncoding { block: vec![
            0,
            0,
            0,
            0,
            1,
            1,
            1,
            1,
            2,
            2,
            2,
            2,
            3,
            3,
            3,
            3,
            4,
            4,
            4,
            4,
            5,
            5,
            5,
            5,
            6,
            6,
            6,
            6,
            7,
            7,
            7,
            7,
            8,
            8,
            8,
            8,
            9,
            9,
            9,
            9,
            10,
            10,
            10,
            10,
            11,
            11,
            11,
            11,
            12,
            12,
            12,
            12,
            13,
            13,
            13,
            13,
            14,
            14,
            14,
            14,
            15,
            15,
            15,
            15,
        ], is_end: true }
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn play_rising_gfsk_encoded_channel_encoding(fixture: TransmitterFixture) {
        let channel_encoding = rising_channel_encoding();
        play_encoding(fixture,  channel_encoding);
    }

    fn play_encoding(fixture: TransmitterFixture, channel_encoding: ChannelEncoding) {
        debug!("Test sending channel encoding");
        fixture.channel_encoding_tx.lock().unwrap().broadcast(channel_encoding);
        debug!("Waiting for transmitter to not be silent");
        while fixture.transmitter.is_silent() {
            test_util::wait_n_ms(250);
        }
        debug!("Transmitter is not silent; waiting for transmitter to finish sending");
        while !fixture.transmitter.is_silent() {
            test_util::wait_n_ms(250);
        }
        debug!("Transmitter is silent; done!");
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn play_rising_gfsk_encoded_channel_encoding_varying_volume(mut fixture: TransmitterFixture) {
        let channel_encoding = rising_channel_encoding();
        fixture.channel_encoding_tx.lock().unwrap().broadcast(channel_encoding);
        debug!("Waiting for transmitter to not be silent");
        while fixture.transmitter.is_silent() {
            test_util::wait_n_ms(250);
        }
        debug!("Transmitter is not silent; varying volume while waiting for transmitter to finish sending");
        let mut amplitude = 1.0;
        let mut amplitude_delta = -0.05;
        while !fixture.transmitter.is_silent() {
            test_util::wait_n_ms(20);
            amplitude += amplitude_delta;
            if amplitude > 1.0 {
                amplitude = 1.0;
                amplitude_delta *= -1.0;
            } else if amplitude < 0.0 {
                amplitude = 0.0;
                amplitude_delta *= -1.0;
            }
            fixture.transmitter.set_amplitude_max(amplitude);
        }
        debug!("Transmitter is silent; done!");
    }
}
