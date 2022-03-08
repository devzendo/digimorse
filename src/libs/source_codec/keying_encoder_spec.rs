use crate::libs::keyer_io::keyer_io::KeyerEdgeDurationMs;

const PERFECT_DIT_DURATION: KeyerEdgeDurationMs = 60;
const PERFECT_DAH_DURATION: KeyerEdgeDurationMs = 180;
const PERFECT_WORDGAP_DURATION: KeyerEdgeDurationMs = 420;

const TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS: usize = 64;

#[cfg(test)]
mod keying_encoder_spec {
    use log::{debug, info};
    use rstest::*;
    use std::env;
    use std::sync::{Arc, RwLock};
    use crate::libs::keyer_io::keyer_io::KeyingTimedEvent;
    use crate::libs::source_codec::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
    use crate::libs::source_codec::keying_encoder::{decode_from_binary, DefaultKeyingEncoder, encode_to_binary, KeyingEncoder};
    use crate::libs::source_codec::keying_encoder::keying_encoder_spec::{PERFECT_DAH_DURATION, PERFECT_DIT_DURATION, PERFECT_WORDGAP_DURATION, TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};
    use crate::libs::source_codec::source_encoding::SourceEncodingBuilder;
    use crate::libs::util::util::dump_byte_vec;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct KeyingEncoderFixture {
        storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>,
        encoder: Box<dyn KeyingEncoder>,
    }

    impl KeyingEncoderFixture {
        pub fn bytes(&mut self) -> Vec<u8> {
            self.storage.write().unwrap().build().block
        }
    }

    #[fixture]
    fn fixture() -> KeyingEncoderFixture {
        let storage: Box<dyn SourceEncodingBuilder + Send + Sync> = Box::new
            (BitvecSourceEncodingBuilder::new(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS));
        let arc_storage = Arc::new(RwLock::new(storage));
        let mut encoder = Box::new(DefaultKeyingEncoder::new(arc_storage.clone()));
        encoder.set_keyer_speed(20);
        KeyingEncoderFixture {
            storage: arc_storage.clone(),
            encoder,
        }
    }

    #[rstest]
    #[should_panic]
    pub fn panic_on_encode_with_no_speed_set(mut fixture: KeyingEncoderFixture) {
        // For convenience, the fixture() sets the speed to 20WPM. Unset it, as though it had not
        // been set...
        fixture.encoder.set_keyer_speed(0);

        fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION });
    }

    #[rstest]
    pub fn encode_perfect_dit(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.bytes(), vec![0b01100000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_perfect_dah(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION }), true);
        assert_eq!(fixture.bytes(), vec![0b01110000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_perfect_wordgap(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION }), true);
        assert_eq!(fixture.bytes(), vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    fn perfect_keying_wont_fit_in_block_so_returns_false(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        // This one won't fit in the block..
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), false);
    }

    #[rstest]
    fn wpm_changes_are_tracked_for_perfect_encodings(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        fixture.encoder.set_keyer_speed(40);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: 30 }), true);
        // No WPM|Polarity encoding at the start or after the change of speed, but the correct
        // perfect duration is updated.
        assert_eq!(fixture.bytes(), vec![0b01100110, 0, 0, 0, 0, 0, 0, 0]);
    }

    // Encoding and decoding of binary numbers -----------------------------------------------------

    #[test]
    #[should_panic]
    pub fn encode_to_binary_lower_range_exceeded() {
        encode_to_binary(-481, 8);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_upper_range_exceeded() {
        encode_to_binary(481, 8);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_lower_bits_range_exceeded() {
        encode_to_binary(0, 4);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_upper_bits_range_exceeded() {
        encode_to_binary(0, 10);
    }

    #[test]
    pub fn encode_to_binary_good_ranges_doesnt_panic() {
        for delta in -480 ..= 480 {
            encode_to_binary(delta, 9);
        }
        for delta in -240 ..= 240 {
            encode_to_binary(delta, 8);
        }
        for delta in -127 ..= 127 {
            encode_to_binary(delta, 7);
        }
        for delta in -63 ..= 63 {
            encode_to_binary(delta, 6);
        }
        for delta in -31 ..= 31 {
            encode_to_binary(delta, 5);
        }

        for bits in 5 ..= 9 {
            encode_to_binary(0, bits);
        }
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_lower_9_bits_range_exceeded() {
        encode_to_binary(-481, 9);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_upper_9_bits_range_exceeded() {
        encode_to_binary(481, 9);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_lower_8_bits_range_exceeded() {
        encode_to_binary(-241, 8);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_upper_8_bits_range_exceeded() {
        encode_to_binary(241, 8);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_lower_7_bits_range_exceeded() {
        encode_to_binary(-128, 7);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_upper_7_bits_range_exceeded() {
        encode_to_binary(128, 7);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_lower_6_bits_range_exceeded() {
        encode_to_binary(-64, 6);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_upper_6_bits_range_exceeded() {
        encode_to_binary(-64, 6);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_lower_5_bits_range_exceeded() {
        encode_to_binary(-32, 5);
    }

    #[test]
    #[should_panic]
    pub fn encode_to_binary_upper_5_bits_range_exceeded() {
        encode_to_binary(32, 5);
    }


    #[test]
    #[should_panic]
    pub fn decode_from_binary_lower_bits_range_exceeded() {
        decode_from_binary(0, 4);
    }

    #[test]
    #[should_panic]
    pub fn decode_from_binary_upper_bits_range_exceeded() {
        decode_from_binary(0, 10);
    }

    #[test]
    pub fn decode_from_binary_lower_output_range_truncated() {
        // dodgy casting..
        assert_eq!(decode_from_binary((-481 as i16) as u16, 9), -480);
    }

    #[test]
    pub fn decode_from_binary_upper_output_range_exceeded() {
        assert_eq!(decode_from_binary(481, 9), 480);
    }

    #[test]
    pub fn encode_to_binary_illustrative_cases() {
        //                                                            Sxxxxx
        assert_eq!(encode_to_binary(-31,   5), 0b0000000000100001);
        assert_eq!(encode_to_binary(-3,    5), 0b0000000000111101);
        assert_eq!(encode_to_binary(-2,    5), 0b0000000000111110);
        assert_eq!(encode_to_binary(-1,    5), 0b0000000000111111);
        assert_eq!(encode_to_binary(0,     5), 0b0000000000000000);
        assert_eq!(encode_to_binary(1,     5), 0b0000000000000001);
        assert_eq!(encode_to_binary(2,     5), 0b0000000000000010);
        assert_eq!(encode_to_binary(3,     5), 0b0000000000000011);
        assert_eq!(encode_to_binary(31,    5), 0b0000000000011111);

        //                                                           Sxxxxxx
        assert_eq!(encode_to_binary(-63,   6), 0b0000000001000001);
        assert_eq!(encode_to_binary(-31,   6), 0b0000000001100001);
        assert_eq!(encode_to_binary(31,    6), 0b0000000000011111);
        assert_eq!(encode_to_binary(63,    6), 0b0000000000111111);

        //                                                          Sxxxxxxx
        assert_eq!(encode_to_binary(-127,  7), 0b0000000010000001);
        assert_eq!(encode_to_binary(-63,   7), 0b0000000011000001);
        assert_eq!(encode_to_binary(63,    7), 0b0000000000111111);
        assert_eq!(encode_to_binary(127,   7), 0b0000000001111111);

        //                                                         Sxxxxxxxx
        assert_eq!(encode_to_binary(-240,  8), 0b0000000100010000);
        assert_eq!(encode_to_binary(-127,  8), 0b0000000110000001);
        assert_eq!(encode_to_binary(127,   8), 0b0000000001111111);
        assert_eq!(encode_to_binary(240,   8), 0b0000000011110000);

        //                                                        Sxxxxxxxxx
        assert_eq!(encode_to_binary(-480,  9), 0b0000001000100000);
        assert_eq!(encode_to_binary(-240,  9), 0b0000001100010000);
        assert_eq!(encode_to_binary(240,   9), 0b0000000011110000);
        assert_eq!(encode_to_binary(480,   9), 0b0000000111100000);
    }

    #[test]
    pub fn encode_to_and_decode_from_binary_round_trip() {
        info!("9 bits");
        for delta in -480 ..= 480 {
            let encoded = encode_to_binary(delta, 9);
            assert_eq!(decode_from_binary(encoded, 9), delta);
        }
        info!("8 bits");
        for delta in -240 ..= 240 {
            let encoded = encode_to_binary(delta, 8);
            assert_eq!(decode_from_binary(encoded, 8), delta);
        }
        info!("7 bits");
        for delta in -127 ..= 127 {
            let encoded = encode_to_binary(delta, 7);
            assert_eq!(decode_from_binary(encoded, 7), delta);
        }
        info!("6 bits");
        for delta in -63 ..= 63 {
            let encoded = encode_to_binary(delta, 6);
            assert_eq!(decode_from_binary(encoded, 6), delta);
        }
        info!("5 bits");
        for delta in -31 ..= 31 {
            let encoded = encode_to_binary(delta, 5);
            assert_eq!(decode_from_binary(encoded, 5), delta);
        }
    }

    // Delta encodings -----------------------------------------------------------------------------

    #[rstest]
    pub fn encode_delta_dit_below(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION - 1 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b10101111, 0b11100000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_dit_above(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION + 1 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b10100000, 0b00100000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_dah_below(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION - 1 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b10111111, 0b11100000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_dah_above(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION + 1 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b10110000, 0b00010000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_wordgap_below(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION - 1 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b11001111, 0b11110000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_wordgap_above(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION + 1 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b11000000, 0b00010000, 0, 0, 0, 0, 0, 0]);
    }


    #[rstest]
    pub fn encode_delta_dit_below_min(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION - 60 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b10101000, 0b10000000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_dit_above_max(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION + 60 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b10100111, 0b10000000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_dah_below_min(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION - 59 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b10111000, 0b10100000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_dah_above_max(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION + 119 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b10110111, 0b01110000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_wordgap_below_min(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION - 120 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b11001000, 0b10000000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_delta_wordgap_above_max(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION + 120 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b11000111, 0b10000000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    fn delta_keying_negative_wont_fit_in_block_so_returns_false(mut fixture: KeyingEncoderFixture) {
        // Using dah here with a negative delta so that the lower-sized encoding (at 20WPM, 6 bits)
        // is used.
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION - 10 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DAH_DURATION - 11 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION - 12 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DAH_DURATION - 13 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION - 14 }), true);
        // This one won't fit in the block..
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DAH_DURATION - 15 }), false);
    }

    #[rstest]
    fn delta_keying_positive_wont_fit_in_block_so_returns_false(mut fixture: KeyingEncoderFixture) {
        // Using dah here with a positive delta so that the higher-sized encoding (at 20WPM, 7 bits)
        // is used.
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION + 10 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DAH_DURATION + 11 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION + 12 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DAH_DURATION + 13 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION + 14 }), true);
        // This one won't fit in the block..
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DAH_DURATION + 15 }), false);
    }

    // Naïve encoding ------------------------------------------------------------------------------

    #[rstest]
    #[should_panic]
    pub fn encode_naive_wordgap_out_of_bounds(mut fixture: KeyingEncoderFixture) {
        fixture.encoder.set_keyer_speed(5);
        fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: 1680 + 368 });
    }

    #[rstest]
    pub fn encode_naive_wordgap(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION + 121 }), true);
        let bytes = fixture.bytes();
        debug!("{}", dump_byte_vec(&bytes));
        assert_eq!(bytes, vec![0b11010100, 0b00111010, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    fn naive_wont_fit_in_block_so_returns_false(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION + 121 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_WORDGAP_DURATION + 122 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION + 123 }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_WORDGAP_DURATION + 124 }), true);
        // This one won't fit in the block..
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION + 125 }), false);
    }

    // TODO what about quantisation?

    // TODO delta wordgap at 5WPM above 367 is encoded as a naïve encoding.
}