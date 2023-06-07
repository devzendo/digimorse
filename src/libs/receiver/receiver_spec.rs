extern crate hamcrest2;

#[cfg(test)]
mod receiver_spec {
    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use bus::Bus;
    use hamcrest2::prelude::*;
    use log::{debug, info};
    use portaudio::PortAudio;
    use rstest::*;

    use crate::libs::audio::audio_devices::open_input_audio_device;
    use crate::libs::receiver::receiver::Receiver;
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

    pub struct ReceiverFixture {
        terminate: Arc<AtomicBool>,
        receiver: Receiver,
        pa: Arc<PortAudio>,
    }

    #[fixture]
    fn fixture() -> ReceiverFixture {
        let terminate = Arc::new(AtomicBool::new(false));

        let audio_frequency = 600 as AudioFrequencyHz;
        info!("Instantiating receiver...");
        let mut receiver = Receiver::new(audio_frequency,
                                               terminate.clone());

        let mut fixture = ReceiverFixture {
            terminate,
            receiver,
            pa: Arc::new(PortAudio::new().unwrap()),
        };
        let input_from_rig = test_hardware::get_current_system_rig_input_name();
        info!("Input from rig '{}'", input_from_rig);
        let input_settings = open_input_audio_device(&fixture.pa, input_from_rig.as_str()).unwrap();
        info!("Setting amplitude max");
        fixture.receiver.set_amplitude_max(1.0 as AmplitudeMax);
        info!("Initialising audio callback...");
        fixture.receiver.start_callback(&fixture.pa, input_settings).unwrap();
        info!("Setting audio frequency...");
        fixture.receiver.set_audio_frequency(audio_frequency);

        info!("Fixture setup sleeping");
        test_util::wait_n_ms(100);
        // give things time to start
        info!("Fixture setup out of sleep");

        fixture
    }

    impl Drop for ReceiverFixture {
        fn drop(&mut self) {
            debug!("ReceiverFixture about to set terminate flag...");
            test_util::wait_n_ms(1000);
            // to detect any clunk when closing streams?
            debug!("ReceiverFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("ReceiverFixture ...set terminate flag");
        }
    }

    #[rstest]
    #[serial]
    #[ignore]
    pub fn not_sure_yet(fixture: ReceiverFixture) {
        info!("Start of test");
        // TODO read a known wav file, inject it, receive it on the receiver's output
        test_util::wait_n_ms(1000);
        info!("End of test");
    }
}
