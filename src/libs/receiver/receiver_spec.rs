extern crate hamcrest2;

#[cfg(test)]
mod receiver_spec {
    use std::env;
    use std::fs::File;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use hamcrest2::prelude::*;
    use log::{debug, info};
    use portaudio::PortAudio;
    use rstest::*;
    use wav::{BitDepth, Header, WAV_FORMAT_IEEE_FLOAT};

    use crate::libs::audio::audio_devices::open_input_audio_device;
    use crate::libs::channel_codec::sample_channel_encoding::sample_channel_encoding;
    use crate::libs::receiver::receiver::Receiver;
    use crate::libs::test::test_hardware;
    use crate::libs::transmitter::modulate::gfsk_modulate;
    use crate::libs::transmitter::transmitter::{AmplitudeMax, AudioFrequencyHz};
    use crate::libs::util::test_util;

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

    #[rstest]
    #[serial]
    #[ignore]
    pub fn receive_sample_waveform(fixture: ReceiverFixture) {
        info!("Start of test");
        // TODO read a known wav file, inject it, receive it on the receiver's output
        let sample_waveform = sample_waveform();
        //fixture.receiver.inject_waveform()
        test_util::wait_n_ms(1000);
        info!("End of test");
    }

    #[test]
    #[serial]
    #[ignore]
    pub fn save_sample_waveform_as_wav() {
        let sample_waveform = sample_waveform();
        let mut out_file = File::create(Path::new("testdata/sample_waveform.wav")).unwrap();
        let header = Header::new(WAV_FORMAT_IEEE_FLOAT, 1, SAMPLE_RATE as u32, 32);
        let data = BitDepth::ThirtyTwoFloat(sample_waveform);
        wav::write(header, &data, &mut out_file).expect("TODO: panic message");
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
