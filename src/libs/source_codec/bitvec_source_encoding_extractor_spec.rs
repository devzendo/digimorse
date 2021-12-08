extern crate hamcrest2;

#[cfg(test)]
mod bitvec_source_encoding_extractor_spec {
    use rstest::*;
    use hamcrest2::prelude::*;
    use log::debug;
    use std::env;
    use crate::libs::source_codec::bitvec_source_encoding_extractor::BitvecSourceEncodingExtractor;
    use crate::libs::source_codec::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
    use crate::libs::source_codec::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, SourceEncodingBuilder, SourceEncodingExtractor};

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
}
