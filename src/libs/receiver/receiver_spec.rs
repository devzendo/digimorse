extern crate hamcrest2;

#[cfg(test)]
mod receiver_spec {
    use std::env;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use bus::{Bus, BusReader};

    use log::{debug, info};
    use portaudio::PortAudio;
    use rstest::*;
    use hamcrest2::prelude::*;
    use crate::libs::application::application::{BusInput, BusOutput};

    use crate::libs::audio::audio_devices::open_input_audio_device;
    use crate::libs::channel_codec::sample_channel_encoding::sample_channel_encoding;
    use crate::libs::receiver::receiver::{Receiver, ReceiverEvent};
    use crate::libs::test::test_hardware;
    use crate::libs::transmitter::modulate::gfsk_modulate;
    use crate::libs::transmitter::transmitter::{AmplitudeMax, AudioFrequencyHz};
    use crate::libs::util::test_util;
    use crate::libs::wav::wav::{read_waveform_file, write_waveform_file};

    const SAMPLE_RATE: AudioFrequencyHz = 48000;
    const AUDIO_FREQUENCY: AudioFrequencyHz = 600;

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

    struct Sampler {

    }

    impl BusInput<ReceiverEvent> for Sampler {
        fn clear_input_rx(&mut self) {
            todo!()
        }

        fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<ReceiverEvent>>>) {
            todo!()
        }
    }

    #[rstest]
    #[serial]
    pub fn receive_sample_waveform(mut fixture: ReceiverFixture) {
        info!("Start of test");
        let receiver_input_bus = Bus::new(16);
        //let reader = receiver_input_bus.add_rx();
        // reader.
        fixture.receiver.set_output_tx(Arc::new(Mutex::new(receiver_input_bus)));

        // TODO read a known wav file, inject it, receive it on the receiver's output
        let filename = "testdata/sample_waveform.wav";
        let sample_waveform = read_waveform_file(filename);
        let waveform_vec = sample_waveform.unwrap();
        assert_that!(waveform_vec.len(), equal_to(MODULATED_SAMPLE_ENCODING_WAVEFORM_LENGTH));
        fixture.receiver.inject_waveform(&waveform_vec);
        // TODO WOZERE - how to sense this?
        panic!("can't sense this");
        test_util::wait_n_ms(1000);
        info!("End of test");
    }

    #[test]
    #[serial]
    #[ignore]
    pub fn save_sample_waveform_as_wav() {
        let sample_waveform = sample_waveform();
        let filename = "testdata/sample_waveform.wav";
        write_waveform_file(sample_waveform, filename).expect("Could not write waveform");
    }

    // length found by running it and seeing what's needed...
    const MODULATED_SAMPLE_ENCODING_WAVEFORM_LENGTH: usize = 493440;
    fn sample_waveform() -> Vec<f32> {
        let channel_encoding = sample_channel_encoding();
        let channel_symbols = &channel_encoding.block;
        let mut waveform_buffer: Vec<f32> = Vec::with_capacity(MODULATED_SAMPLE_ENCODING_WAVEFORM_LENGTH);
        waveform_buffer.resize(MODULATED_SAMPLE_ENCODING_WAVEFORM_LENGTH, 0_f32);

        let _ = gfsk_modulate(AUDIO_FREQUENCY, SAMPLE_RATE, channel_symbols, &mut waveform_buffer.as_mut_slice(), true, true);
        waveform_buffer
    }
}
