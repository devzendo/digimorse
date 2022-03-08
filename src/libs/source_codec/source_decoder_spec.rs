extern crate hamcrest2;

#[cfg(test)]
mod source_decoder_spec {
    use log::{debug, info};
    use std::env;
    use crate::libs::source_codec::source_decoder::source_decode;
    use crate::libs::source_codec::source_encoding::Frame;
    use crate::libs::source_codec::test_encoding_builder::encoded;
    use crate::libs::util::util::dump_byte_vec;

    const TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS: usize = 64;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    pub fn decode_emptiness() {
        // Looks like Padding!
        let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[]);
        let expected_frames = vec![Frame::Padding];
        assert_decoded_eq(block, expected_frames)
    }

    #[test]
    pub fn decode_complete_emptiness() {
        should_decode_with_error(vec![], "Cannot decode a block of the wrong size")
    }

    #[test]
    pub fn decode_wrong_size() {
        should_decode_with_error(vec![0, 0], "Cannot decode a block of the wrong size");
    }

    #[test]
    pub fn first_padding_skips_other_stuff() {
        let expected_frames = &[
            Frame::Padding,
        ];
        // Can't use encoded() here since when you ask it to add a Padding, it fills the rest of
        // the block with 0's, which we want to circumvent here.
        let hand_coded_block = vec![0b00000101, 0, 0, 0, 0, 0, 0, 0];
        assert_decoded_eq(hand_coded_block, expected_frames.to_vec());
    }

    #[test]
    pub fn keying_without_wpmpolarity_cannot_be_decoded() {
        let frames = vec![Frame::KeyingPerfectDit, Frame::KeyingPerfectDah, Frame::KeyingPerfectWordgap,
                          Frame::KeyingDeltaDit { delta: 2 }, Frame::KeyingDeltaDah { delta: 2}, Frame::KeyingDeltaWordgap { delta: 4},
            Frame::KeyingNaive { duration: 100 },
            Frame::KeyingEnd
        ];
        // KeyingNaive doesn't need WPM, but it needs polarity to ensure the correct KeyingEvents are emitted.
        for frame in frames {
            let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
                frame,
            ]);
            should_decode_with_error(block, "Cannot decode keying without prior WPM|Polarity")
        }
    }

    // Tests for specific frames, which all have to be surrounded by a WPMPolarity and Padding.

    #[test]
    pub fn decode_wpm_polarity() {
        let keying_frames = &[
            Frame::WPMPolarity { wpm: 20, polarity: true },
            Frame::Padding
        ];
        let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, keying_frames);
        assert_decoded_eq(block, keying_frames.to_vec());
    }

    #[test]
    pub fn decode_perfect_dit() {
        assert_decoded_frame(Frame::KeyingPerfectDit);
    }

    #[test]
    pub fn decode_perfect_dah() {
        assert_decoded_frame(Frame::KeyingPerfectDah);
    }

    #[test]
    pub fn decode_perfect_wordgap() {
        assert_decoded_frame(Frame::KeyingPerfectWordgap);
    }

    // Decode delta tests also prove that the WPMPolarity speed is given to the KeyingTiming, as if
    // if it wasn't, it'd panic with WPM == 0.

    #[test]
    pub fn decode_delta_dit() {
        assert_decoded_frame(Frame::KeyingDeltaDit { delta: -5 });
        assert_decoded_frame(Frame::KeyingDeltaDit { delta: 5 });
    }

    #[test]
    pub fn decode_delta_dah() {
        assert_decoded_frame(Frame::KeyingDeltaDah { delta: -15 });
        assert_decoded_frame(Frame::KeyingDeltaDah { delta: 15 });
    }

    #[test]
    pub fn decode_delta_wordgap() {
        assert_decoded_frame(Frame::KeyingDeltaWordgap { delta: -100 });
        assert_decoded_frame(Frame::KeyingDeltaWordgap { delta: 100 });
    }

    #[test]
    pub fn decode_naive() {
        assert_decoded_frame(Frame::KeyingNaive { duration: 80 });
    }

    #[test]
    pub fn decode_unused() {
        assert_decoded_frame(Frame::Unused);
    }

    #[test]
    pub fn decode_extension() {
        assert_decoded_frame(Frame::Extension);
    }

    #[test]
    pub fn wpm_polarity_causes_timing_recalculation() {
        let keying_frames = &[
            Frame::WPMPolarity { wpm: 5, polarity: true },
            Frame::KeyingDeltaDah { delta: 5 },
            Frame::WPMPolarity { wpm: 60, polarity: true },
            Frame::KeyingDeltaDah { delta: 5 },
            Frame::Extension, // It stands out as 1111 in the debug output below.
            Frame::Padding
        ];
        let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, keying_frames);
        debug!("{}", dump_byte_vec(&block));

        // encoded tracks changes in speed when WPMPolarity frames are
        // encoded, causing changes in length in the delta dah frames.
        // They'll be correctly round-tripped here.
        assert_decoded_eq(block, keying_frames.to_vec());
    }

    fn should_decode_with_error(block: Vec<u8>, expected_error_message: &str) {
        match source_decode(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, block) {
            Ok(_) => {
                panic!("Should not have successfully decoded")
            }
            Err(e) => {
                info!("Expected error: {}", e);
                assert_eq!(e.to_string(), expected_error_message);
            }
        }
    }

    fn assert_decoded_eq(block: Vec<u8>, expected_frames: Vec<Frame>) {
        match source_decode(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, block) {
            Ok(frames) => {
                assert_eq!(frames, expected_frames);
            }
            Err(e) => {
                panic!("Should not fail with {}", e);
            }
        }
    }

    fn assert_decoded_frame(frame: Frame) {
        let keying_frames = &[
            Frame::WPMPolarity { wpm: 20, polarity: true },
            frame,
            Frame::Padding
        ];
        let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, keying_frames);
        assert_decoded_eq(block, keying_frames.to_vec());
    }
}
