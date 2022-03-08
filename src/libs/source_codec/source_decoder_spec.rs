extern crate hamcrest2;

#[cfg(test)]
mod source_decoder_spec {
    use log::{debug, info};
    use std::env;
    use crate::libs::source_codec::source_decoder::SourceDecoder;
    use crate::libs::source_codec::source_encoding::Frame;
    use crate::libs::source_codec::test_encoding_builder::encoded;
    use crate::libs::util::util::dump_byte_vec;
    use rstest::*;

    const TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS: usize = 64;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct SourceDecoderFixture {
        source_decoder: SourceDecoder,
    }

    #[fixture]
    fn fixture() -> SourceDecoderFixture {
        let source_decoder = SourceDecoder::new(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        SourceDecoderFixture {
            source_decoder
        }
    }

    #[test]
    #[should_panic]
    fn block_size_must_be_a_multiple_of_8_not_0() {
        try_block_size(0);
    }

    #[test]
    #[should_panic]
    fn block_size_must_be_a_multiple_of_8_not_7() {
        try_block_size(7);
    }

    #[test]
    #[should_panic]
    fn block_size_must_be_a_multiple_of_8_not_9() {
        try_block_size(9);
    }

    #[test]
    fn block_size_must_be_a_multiple_of_8() {
        try_block_size(8);
    }

    fn try_block_size(block_size: usize) {
        let _ = SourceDecoder::new(block_size);
    }

    #[rstest]
    pub fn decode_emptiness(fixture: SourceDecoderFixture) {
        // Looks like Padding!
        let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[]);
        let expected_frames = vec![Frame::Padding];
        assert_decoded_eq(&fixture, block, expected_frames)
    }

    #[rstest]
    pub fn decode_complete_emptiness(fixture: SourceDecoderFixture) {
        should_decode_with_error(&fixture, vec![], "Cannot decode a block of the wrong size")
    }

    #[rstest]
    pub fn decode_wrong_size(fixture: SourceDecoderFixture) {
        should_decode_with_error(&fixture, vec![0, 0], "Cannot decode a block of the wrong size");
    }

    #[rstest]
    pub fn first_padding_skips_other_stuff(fixture: SourceDecoderFixture) {
        let expected_frames = &[
            Frame::Padding,
        ];
        // Can't use encoded() here since when you ask it to add a Padding, it fills the rest of
        // the block with 0's, which we want to circumvent here.
        let hand_coded_block = vec![0b00000101, 0, 0, 0, 0, 0, 0, 0];
        assert_decoded_eq(&fixture, hand_coded_block, expected_frames.to_vec());
    }

    #[rstest]
    pub fn keying_without_wpmpolarity_cannot_be_decoded(fixture: SourceDecoderFixture) {
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
            should_decode_with_error(&fixture, block, "Cannot decode keying without prior WPM|Polarity")
        }
    }

    // Tests for specific frames, which all have to be surrounded by a WPMPolarity and Padding.
    #[rstest]
    pub fn decode_wpm_polarity(fixture: SourceDecoderFixture) {
        let keying_frames = &[
            Frame::WPMPolarity { wpm: 20, polarity: true },
            Frame::Padding
        ];
        let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, keying_frames);
        assert_decoded_eq(&fixture, block, keying_frames.to_vec());
    }

    #[rstest]
    pub fn decode_perfect_dit(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::KeyingPerfectDit);
    }

    #[rstest]
    pub fn decode_perfect_dah(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::KeyingPerfectDah);
    }

    #[rstest]
    pub fn decode_perfect_wordgap(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::KeyingPerfectWordgap);
    }

    // Decode delta tests also prove that the WPMPolarity speed is given to the KeyingTiming, as if
    // if it wasn't, it'd panic with WPM == 0.

    #[rstest]
    pub fn decode_delta_dit(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::KeyingDeltaDit { delta: -5 });
        assert_decoded_frame(&fixture, Frame::KeyingDeltaDit { delta: 5 });
    }

    #[rstest]
    pub fn decode_delta_dah(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::KeyingDeltaDah { delta: -15 });
        assert_decoded_frame(&fixture, Frame::KeyingDeltaDah { delta: 15 });
    }

    #[rstest]
    pub fn decode_delta_wordgap(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::KeyingDeltaWordgap { delta: -100 });
        assert_decoded_frame(&fixture, Frame::KeyingDeltaWordgap { delta: 100 });
    }

    #[rstest]
    pub fn decode_naive(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::KeyingNaive { duration: 80 });
    }

    #[rstest]
    pub fn decode_unused(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::Unused);
    }

    #[rstest]
    pub fn decode_extension(fixture: SourceDecoderFixture) {
        assert_decoded_frame(&fixture, Frame::Extension);
    }

    #[rstest]
    pub fn wpm_polarity_causes_timing_recalculation(fixture: SourceDecoderFixture) {
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
        assert_decoded_eq(&fixture, block, keying_frames.to_vec());
    }

    fn should_decode_with_error(fixture: &SourceDecoderFixture, block: Vec<u8>, expected_error_message: &str) {
        match fixture.source_decoder.source_decode(block) {
            Ok(_) => {
                panic!("Should not have successfully decoded")
            }
            Err(e) => {
                info!("Expected error: {}", e);
                assert_eq!(e.to_string(), expected_error_message);
            }
        }
    }

    fn assert_decoded_eq(fixture: &SourceDecoderFixture, block: Vec<u8>, expected_frames: Vec<Frame>) {
        match fixture.source_decoder.source_decode(block) {
            Ok(frames) => {
                assert_eq!(frames, expected_frames);
            }
            Err(e) => {
                panic!("Should not fail with {}", e);
            }
        }
    }

    fn assert_decoded_frame(fixture: &SourceDecoderFixture, frame: Frame) {
        let keying_frames = &[
            Frame::WPMPolarity { wpm: 20, polarity: true },
            frame,
            Frame::Padding
        ];
        let block = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, keying_frames);
        assert_decoded_eq(fixture, block, keying_frames.to_vec());
    }
}
