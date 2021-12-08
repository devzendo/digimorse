extern crate hamcrest2;

#[cfg(test)]
mod bitvec_source_encoding_extractor_spec {
    use log::debug;
    use std::env;
    use crate::libs::source_codec::bitvec_source_encoding_extractor::BitvecSourceEncodingExtractor;
    use crate::libs::source_codec::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, SourceEncodingExtractor};

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    fn extractor(source: Vec<u8>) -> Box<dyn SourceEncodingExtractor> {
        Box::new(BitvecSourceEncodingExtractor::new(source))
    }

    #[test]
    #[should_panic]
    pub fn empty_source() {
        extractor(vec![]);
    }

    #[test]
    #[should_panic]
    pub fn wrong_size_source() {
        extractor(vec![0, 0, 0,]);
    }

    #[test]
    pub fn right_size_source() {
        extractor(vec![0, 0, 0, 0, 0, 0, 0, 0]);
        // doesn't panic
    }

    #[test]
    pub fn full_source_has_all_bits_remaining() {
        let extractor = extractor(vec![0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
    }

    // extract_bool --------------------------------------------------------------------------------

    #[test]
    pub fn extract_bool_reduces_remaining() {
        let mut extractor = extractor(vec![0b10100000, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.extract_bool(), true);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 1);
        assert_eq!(extractor.extract_bool(), false);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 2);
        assert_eq!(extractor.extract_bool(), true);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 3);
    }

    #[test]
    #[should_panic]
    pub fn extract_bool_extract_too_much() {
        let mut extractor = extractor(vec![0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        extractor.extract_32_bits(32);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 32);
        extractor.extract_32_bits(32);
        assert_eq!(extractor.remaining(), 0);
        extractor.extract_bool(); // boom
    }

    // extract_8_bits ------------------------------------------------------------------------------

    #[test]
    pub fn extract_8_by_zero_keeps_remaining() {
        let mut extractor = extractor(vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.extract_8_bits(0), 0b00000000);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
    }

    #[test]
    pub fn extract_8_by_a_bit_reduces_remaining() {
        let mut extractor = extractor(vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.extract_8_bits(1), 0b00000001);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 1);
    }

    #[test]
    pub fn extract_8_by_4_bits_reduces_remaining() {
        let mut extractor = extractor(vec![0b10110000, 0, 0, 0, 0, 0, 0, 0]);
        let extracted = extractor.extract_8_bits(4);
        debug!("test extracted {:#010b}", extracted);
        assert_eq!(extracted, 0b00001011);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 4);
    }

    #[test]
    pub fn extract_8_by_8_bits_retains_bit_ordering() {
        let mut extractor = extractor(vec![0b10110001, 0, 0, 0, 0, 0, 0, 0]);
        let extracted = extractor.extract_8_bits(8);
        debug!("test extracted {:#010b}", extracted);

        assert_eq!(extracted, 0b10110001);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 8);
    }

    #[test]
    #[should_panic]
    pub fn extract_8_too_much_panics() {
        let mut extractor = extractor(vec![0b10110001, 0, 0, 0, 0, 0, 0, 0]);
        extractor.extract_8_bits(9);
    }

    #[test]
    #[should_panic]
    pub fn extract_8_extract_too_much() {
        let mut extractor = extractor(vec![0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        extractor.extract_8_bits(8);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 8);
        extractor.extract_8_bits(8);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 16);
        extractor.extract_8_bits(8);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 24);
        extractor.extract_8_bits(8);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 32);
        extractor.extract_8_bits(8);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 40);
        extractor.extract_8_bits(8);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 48);
        extractor.extract_8_bits(8);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 56 );
        extractor.extract_8_bits(8);
        assert_eq!(extractor.remaining(), 0);
        extractor.extract_8_bits(1); // boom
    }

    // extract_16_bits -----------------------------------------------------------------------------

    #[test]
    pub fn extract_16_by_zero_keeps_remaining() {
        let mut extractor = extractor(vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.extract_16_bits(0), 0b0000000000000000);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
    }

    #[test]
    pub fn extract_16_by_a_bit_reduces_remaining() {
        let mut extractor = extractor(vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.extract_16_bits(1), 0b0000000000000001);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 1);
    }

    #[test]
    pub fn extract_16_by_4_bits_reduces_remaining() {
        let mut extractor = extractor(vec![0b10110000, 0, 0, 0, 0, 0, 0, 0]);
        let extracted = extractor.extract_16_bits(4);
        debug!("test extracted {:#018b}", extracted);
        assert_eq!(extracted, 0b0000000000001011);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 4);
    }

    #[test]
    pub fn extract_16_by_8_bits_retains_bit_ordering() {
        let mut extractor = extractor(vec![0b10110001, 0, 0, 0, 0, 0, 0, 0]);
        let extracted = extractor.extract_16_bits(8);
        debug!("test extracted {:#018b}", extracted);

        assert_eq!(extracted, 0b0000000010110001);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 8);
    }

    #[test]
    pub fn extract_16_by_16_bits_retains_bit_ordering() {
        let mut extractor = extractor(vec![0b10110001, 0b00110011, 0, 0, 0, 0, 0, 0]);
        let extracted = extractor.extract_16_bits(16);
        debug!("test extracted {:#018b}", extracted);

        assert_eq!(extracted, 0b1011000100110011);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 16);
    }

    #[test]
    #[should_panic]
    pub fn extract_16_too_much_panics() {
        let mut extractor = extractor(vec![0b10110001, 0, 0, 0, 0, 0, 0, 0]);
        extractor.extract_16_bits(17);
    }

    #[test]
    #[should_panic]
    pub fn extract_16_extract_too_much() {
        let mut extractor = extractor(vec![0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        extractor.extract_16_bits(16);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 16);
        extractor.extract_16_bits(16);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 32);
        extractor.extract_16_bits(16);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 48);
        extractor.extract_16_bits(16);
        assert_eq!(extractor.remaining(), 0);
        extractor.extract_16_bits(1); // boom
    }

    // extract_32_bits -----------------------------------------------------------------------------

    #[test]
    pub fn extract_32_by_zero_keeps_remaining() {
        let mut extractor = extractor(vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.extract_32_bits(0), 0b0000000000000000);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
    }

    #[test]
    pub fn extract_32_by_a_bit_reduces_remaining() {
        let mut extractor = extractor(vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.extract_32_bits(1), 0b00000000000000000000000000000001);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 1);
    }

    #[test]
    pub fn extract_32_by_4_bits_reduces_remaining() {
        let mut extractor = extractor(vec![0b10110000, 0, 0, 0, 0, 0, 0, 0]);
        let extracted = extractor.extract_32_bits(4);
        debug!("test extracted {:#034b}", extracted);
        assert_eq!(extracted, 0b00000000000000000000000000001011);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 4);
    }

    #[test]
    pub fn extract_32_by_8_bits_retains_bit_ordering() {
        let mut extractor = extractor(vec![0b10110001, 0, 0, 0, 0, 0, 0, 0]);
        let extracted = extractor.extract_32_bits(8);
        debug!("test extracted {:#034b}", extracted);

        assert_eq!(extracted, 0b00000000000000000000000010110001);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 8);
    }

    #[test]
    pub fn extract_32_by_16_bits_retains_bit_ordering() {
        let mut extractor = extractor(vec![0b10110001, 0b00110011, 0, 0, 0, 0, 0, 0]);
        let extracted = extractor.extract_32_bits(16);
        debug!("test extracted {:#034b}", extracted);

        assert_eq!(extracted, 0b00000000000000001011000100110011);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 16);
    }

    #[test]
    pub fn extract_32_by_32_bits_retains_bit_ordering() {
        let mut extractor = extractor(vec![0b10110001, 0b00110011, 0b01010101, 0b11110000, 0, 0, 0, 0]);
        let extracted = extractor.extract_32_bits(32);
        debug!("test extracted {:#034b}", extracted);

        assert_eq!(extracted, 0b10110001001100110101010111110000);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 32);
    }

    #[test]
    #[should_panic]
    pub fn extract_32_too_much_panics() {
        let mut extractor = extractor(vec![0b10110001, 0, 0, 0, 0, 0, 0, 0]);
        extractor.extract_32_bits(33);
    }

    #[test]
    #[should_panic]
    pub fn extract_32_extract_too_much() {
        let mut extractor = extractor(vec![0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        extractor.extract_32_bits(32);
        assert_eq!(extractor.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 32);
        extractor.extract_32_bits(32);
        assert_eq!(extractor.remaining(), 0);
        extractor.extract_32_bits(1); // boom
    }
}
