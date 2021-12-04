extern crate hamcrest2;

#[cfg(test)]
mod bitvec_source_encoding_extractor_spec {
    use rstest::*;
    use hamcrest2::prelude::*;
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
    }
}
