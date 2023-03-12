extern crate hamcrest2;

#[cfg(test)]
mod audio_devices_spec {
    use std::env;
    use hamcrest2::prelude::*;
    use crate::libs::audio::audio_devices;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    pub fn parse_dev_name() {
        let (maybe_idx, name) = audio_devices::parse_dev_name("speaker").unwrap();
        assert_that!(maybe_idx, none());
        assert_that!(name, equal_to("speaker"));
    }

    #[test]
    pub fn parse_index_and_dev_name() {
        let (maybe_idx, name) = audio_devices::parse_dev_name("3:microphone").unwrap();
        assert_that!(maybe_idx, has(3));
        assert_that!(name, equal_to("microphone"));
    }

    #[test]
    pub fn parse_index_and_dev_name_with_spaces() {
        let (maybe_idx, name) = audio_devices::parse_dev_name("3 : microphone").unwrap();
        assert_that!(maybe_idx, has(3));
        assert_that!(name, equal_to("microphone"));
    }

    #[test]
    pub fn parse_missing_index() {
        match audio_devices::parse_dev_name(":microphone") {
            Ok(_) => {
                panic!("Not meant to get an Ok for this")
            }
            Err(e) => {
                assert_that!(e.to_string(), equal_to("Missing device index number at start of ':microphone'"))
            }
        }
    }
}