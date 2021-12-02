extern crate hamcrest2;

#[cfg(test)]
mod source_decoder_spec {
    use std::env;
    use crate::libs::source_codec::source_decoder::source_decode;
    use crate::libs::source_codec::source_encoding::Frame;
    use crate::libs::source_codec::test_encoding_builder::encoded;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    pub fn decode_emptiness() {
        let block = encoded(20, &[]);
        let expected_frames = vec![];
        assert_decoded_eq(block, expected_frames)
    }

    fn assert_decoded_eq(block: Vec<u8>, expected_frames: Vec<Frame>) {
        match source_decode(block) {
            Ok(frames) => {
                assert_eq!(frames, expected_frames);
            }
            Err(e) => {
                panic!("Should not fail with {}", e);
            }
        }
    }
}
