#[cfg(test)]
mod modulate_spec {
    use crate::libs::channel_codec::sample_channel_encoding::sample_channel_encoding;
    use crate::libs::transmitter::modulate::gfsk_modulate;
    use crate::libs::transmitter::transmitter::AudioFrequencyHz;

    const SAMPLE_RATE: AudioFrequencyHz = 48000;
    const AUDIO_FREQUENCY: AudioFrequencyHz = 600;

    #[ctor::ctor]
    fn before_each() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    #[should_panic]
    fn empty_panic() {
        let channel_symbols = &sample_channel_encoding().block;
        let mut empty_f32: [f32; 0] = [];
        gfsk_modulate(AUDIO_FREQUENCY, SAMPLE_RATE, channel_symbols, &mut empty_f32, true, true);
    }

}
