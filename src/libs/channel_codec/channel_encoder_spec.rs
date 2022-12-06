extern crate hamcrest2;

#[cfg(test)]
mod channel_encoder_spec {
    use std::env;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use bus::{Bus, BusReader};
    use hamcrest2::prelude::*;

    use log::{debug, info};
    use rstest::*;
    use crate::libs::application::application::{BusInput, BusOutput};
    use crate::libs::channel_codec::channel_encoder::{ChannelEncoder, source_encoding_to_channel_encoding};
    use crate::libs::channel_codec::channel_encoding::{ChannelEncoding, ChannelSymbol};
    use crate::libs::source_codec::source_encoding::{Frame, SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, SourceEncoding};
    use crate::libs::source_codec::test_encoding_builder::encoded;

    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct ChannelEncoderFixture {
        terminate: Arc<AtomicBool>,
        source_encoding_tx: Bus<SourceEncoding>,
        channel_encoder_rx: BusReader<ChannelEncoding>,
        channel_encoder: ChannelEncoder,
    }

    #[fixture]
    fn fixture() -> ChannelEncoderFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut source_encoding_tx = Bus::new(16);
        let source_encoding_rx = source_encoding_tx.add_rx();
        let mut channel_encoder_tx = Bus::new(16);
        let channel_encoder_rx = channel_encoder_tx.add_rx();
        let mut channel_encoder = ChannelEncoder::new(source_encoding_to_channel_encoding, terminate.clone());
        channel_encoder.set_input_rx(Arc::new(Mutex::new(source_encoding_rx)));
        channel_encoder.set_output_tx(Arc::new(Mutex::new(channel_encoder_tx)));

        info!("Fixture setup sleeping");
        test_util::wait_5_ms();
        // give things time to start
        info!("Fixture setup out of sleep");

        ChannelEncoderFixture {
            terminate,
            source_encoding_tx,
            channel_encoder_rx,
            channel_encoder
        }
    }

    impl Drop for ChannelEncoderFixture {
        fn drop(&mut self) {
            debug!("ChannelEncoderFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("ChannelEncoderFixture ...set terminate flag");
        }
    }

    #[rstest]
    pub fn transform_encodings_with_channel_encoder_active_object(mut fixture: ChannelEncoderFixture) {
        let source_encoding = generate_sample_source_encoding();
        fixture.source_encoding_tx.broadcast(source_encoding);
        info!("source encoding sent; waiting for channel encoding");

        let result = fixture.channel_encoder_rx.recv_timeout(Duration::from_millis(250));
        let expected_channel_encoding = generate_expected_channel_encoding();
        info!("channel encoding is {}", result.clone().unwrap());
        assert_that!(result, has(expected_channel_encoding));
    }

    #[test]
    pub fn transform_encodings_with_channel_encoder_function() {
        let source_encoding = generate_sample_source_encoding();
        let channel_encoding = source_encoding_to_channel_encoding(source_encoding);
        let channel_encoding_clone = channel_encoding.clone();
        for line in channel_encoding_clone.block {
            debug!("Channel encoding {:?}", line);
        }
        let expected_channel_encoding = generate_expected_channel_encoding();
        assert_that!(channel_encoding, equal_to(expected_channel_encoding));
    }


    fn generate_sample_source_encoding() -> SourceEncoding {
        let keying_frames = &[
            Frame::WPMPolarity { wpm: 5, polarity: true },
            Frame::KeyingDeltaDah { delta: 5 },
            Frame::WPMPolarity { wpm: 60, polarity: true },
            Frame::KeyingDeltaDah { delta: 5 },
            Frame::Extension, // It stands out as 1111 in the debug output below.
            Frame::Padding
        ];
        let block = encoded(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, keying_frames);
        let source_encoding = SourceEncoding { block, is_end: true };
        source_encoding
    }

    fn generate_expected_channel_encoding() -> ChannelEncoding {
        ChannelEncoding { block: vec![
            ChannelSymbol::RampUp,
            ChannelSymbol::Tone { value: 1 },
            ChannelSymbol::Tone { value: 1 },
            ChannelSymbol::Tone { value: 4 },
            ChannelSymbol::Tone { value: 5 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 3 },
            ChannelSymbol::Tone { value: 12 },
            ChannelSymbol::Tone { value: 8 },
            ChannelSymbol::Tone { value: 13 },
            ChannelSymbol::Tone { value: 14 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 14 },
            ChannelSymbol::Tone { value: 9 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 3 },
            ChannelSymbol::Tone { value: 8 },
            ChannelSymbol::Tone { value: 15 },
            ChannelSymbol::Tone { value: 1 },
            ChannelSymbol::Tone { value: 2 },
            ChannelSymbol::Tone { value: 11 },
            ChannelSymbol::Tone { value: 13 },
            ChannelSymbol::Tone { value: 11 },
            ChannelSymbol::Tone { value: 4 },
            ChannelSymbol::Tone { value: 10 },
            ChannelSymbol::Tone { value: 3 },
            ChannelSymbol::Tone { value: 4 },
            ChannelSymbol::Tone { value: 10 },
            ChannelSymbol::Tone { value: 10 },
            ChannelSymbol::Tone { value: 9 },
            ChannelSymbol::Tone { value: 6 },
            ChannelSymbol::Tone { value: 7 },
            ChannelSymbol::Tone { value: 8 },
            ChannelSymbol::Tone { value: 10 },
            ChannelSymbol::Tone { value: 9 },
            ChannelSymbol::Tone { value: 1 },
            ChannelSymbol::Tone { value: 13 },
            ChannelSymbol::Tone { value: 14 },
            ChannelSymbol::Tone { value: 5 },
            ChannelSymbol::Tone { value: 2 },
            ChannelSymbol::Tone { value: 3 },
            ChannelSymbol::Tone { value: 1 },
            ChannelSymbol::Tone { value: 5 },
            ChannelSymbol::Tone { value: 8 },
            ChannelSymbol::Tone { value: 5 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 0 },
            ChannelSymbol::Tone { value: 10 },
            ChannelSymbol::Tone { value: 9 },
            ChannelSymbol::Tone { value: 2 },
            ChannelSymbol::Tone { value: 3 },
            ChannelSymbol::RampDown
        ], is_end: true }
    }
}
