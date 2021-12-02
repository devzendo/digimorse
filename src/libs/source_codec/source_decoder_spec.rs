extern crate hamcrest2;

#[cfg(test)]
mod source_decoder_spec {
    use log::info;
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

    #[test]
    pub fn decode_complete_emptiness() {
        should_decode_with_error(vec![])
    }

    #[test]
    pub fn decode_wrong_size() {
        should_decode_with_error(vec![0, 0]);
    }

    fn should_decode_with_error(block: Vec<u8>) {
        match source_decode(block) {
            Ok(frames) => {
                panic!("Should not have successfully decoded")
            }
            Err(e) => {
                info!("Expected error: {}", e);
            }
        }
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
