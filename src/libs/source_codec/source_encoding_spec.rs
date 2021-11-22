extern crate hamcrest2;

#[cfg(test)]
mod source_encoding_spec {
    use std::env;
    use temp_testdir::TempDir;
    use crate::libs::config_file::config_file::ConfigurationStore;
    use hamcrest2::prelude::*;
    use std::path::Path;
    use crate::libs::keyer_io::keyer_io::KeyerType;
    use crate::libs::source_codec::source_encoding::EncoderFrameType;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn encoder_frame_type_values() {
        assert_eq!(EncoderFrameType::Padding as u32, 0);
        assert_eq!(EncoderFrameType::WPMPolarity as u32, 1);
        assert_eq!(EncoderFrameType::CallsignMetadata as u32, 2);
        // ...
        assert_eq!(EncoderFrameType::KeyingPerfectDit as u32, 6);
        // ...
        assert_eq!(EncoderFrameType::Unused as u32, 14);
        assert_eq!(EncoderFrameType::Extension as u32, 15);
    }
}
