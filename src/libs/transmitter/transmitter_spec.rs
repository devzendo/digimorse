extern crate hamcrest2;

// These are all manually run (and asserted correct aurally and with Audio Hijack / spectrum analyser).
#[cfg(test)]
mod transmitter_spec {
    use bus::Bus;
    use log::{debug, info};
    use std::env;
    use rstest::*;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use hamcrest2::prelude::*;
    use crate::libs::audio::audio_devices::open_output_audio_device;
    use crate::libs::util::test_util;
    use portaudio as pa;
    use portaudio::PortAudio;
    use crate::libs::application::application::BusInput;
    use crate::libs::channel_codec::channel_encoding::ChannelEncoding;
    use crate::libs::channel_codec::sample_channel_encoding::sample_channel_encoding;
    use crate::libs::transmitter::transmitter::{AudioFrequencyKHz, Transmitter};

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

    #[fixture]
    fn fixture() -> TransmitterFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut channel_encoding_tx = Bus::new(16);
        let channel_encoding_rx = channel_encoding_tx.add_rx();
        let fixture_channel_encoding_tx = Arc::new(Mutex::new(channel_encoding_tx));

        let old_macbook = true;
        let dev = if old_macbook {"Built-in Output"} else {"MacBook Pro Speakers"};
        let audio_frequency = 600 as AudioFrequencyKHz;
        info!("Instantiating transmitter...");
        let transmitter_channel_encoding_rx = Arc::new(Mutex::new(channel_encoding_rx));
        let mut transmitter = Transmitter::new(audio_frequency,
                                                 terminate.clone());
        transmitter.set_input_rx(transmitter_channel_encoding_rx);

        info!("Setting audio freqency...");
        transmitter.set_audio_frequency(audio_frequency);

        let mut fixture = TransmitterFixture {
            terminate,
            channel_encoding_tx: fixture_channel_encoding_tx,
            transmitter,
            pa: Arc::new(pa::PortAudio::new().unwrap()),
        };
        let output_settings = open_output_audio_device(&fixture.pa, dev).unwrap();
        info!("Initialising audio callback...");
        fixture.transmitter.start_callback(&fixture.pa, output_settings).unwrap();

        info!("Fixture setup sleeping");
        test_util::wait_n_ms(100); // give things time to start
        info!("Fixture setup out of sleep");

        fixture
    }

    impl Drop for TransmitterFixture {
        fn drop(&mut self) {
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
    pub fn play_gfsk_encoded_channel_encoding(fixture: TransmitterFixture) {
        let channel_encoding = sample_channel_encoding();
        debug!("Test sending channel encoding");
        fixture.channel_encoding_tx.lock().unwrap().broadcast(channel_encoding);
        debug!("Waiting for transmitter to send");
        test_util::wait_5_ms();
        assert_that!(fixture.transmitter.is_silent(), equal_to(true));
        debug!("Waiting for transmitter to finish sending");
        while !fixture.transmitter.is_silent() {
            test_util::wait_5_ms();
        }
        debug!("Done!");
    }
}
