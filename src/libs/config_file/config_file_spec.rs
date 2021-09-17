extern crate hamcrest2;

#[cfg(test)]
mod config_file_spec {
    use std::env;
    use temp_testdir::TempDir;
    use crate::libs::config_file::config_file::ConfigurationStore;
    use hamcrest2::prelude::*;
    use std::path::Path;
    use crate::libs::keyer_io::keyer_io::KeyerType;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    fn temp_config_dir() -> (Box<Path>, TempDir) {
        // Return both objects as if temp_dir is not moved back to the caller, it'll drop and
        // delete.
        let temp_dir = TempDir::default();
        let temp = temp_dir.to_path_buf();
        assert_that!(temp.as_path(), dir_exists());

        (temp.into_boxed_path(), temp_dir)
    }

    #[test]
    fn new_config_creates_a_toml_file() {
        let (temp, _temp_dir) = temp_config_dir();
        let config = ConfigurationStore::new(temp).unwrap();

        let config_file_path = config.get_config_file_path();
        assert_that!(config_file_path, path_exists());
        assert_that!(config_file_path, file_exists());
        assert_that!(config_file_path.to_string_lossy(), matches_regex("digimorse.toml$"));
    }

    #[test]
    fn default_settings() {
        let (temp, _temp_dir) = temp_config_dir();
        let config = ConfigurationStore::new(temp.clone()).unwrap();

        assert_that!(config.get_keyer_type(), eq(KeyerType::Null));
        assert_that!(config.get_port(), eq(""));
        assert_that!(config.get_wpm(), eq(20));
        assert_that!(config.get_audio_out_device(), eq(""));
        assert_that!(config.get_rig_out_device(), eq(""));
        assert_that!(config.get_rig_in_device(), eq(""));
    }

    #[test]
    fn settings_can_be_changed_persisted_and_reloaded() {
        let (temp, _temp_dir) = temp_config_dir();
        let mut config = ConfigurationStore::new(temp.clone()).unwrap();

        config.set_keyer_type(KeyerType::Arduino).unwrap();
        config.set_port("/dev/imaginary-usb-port".to_string()).unwrap();
        config.set_wpm(40).unwrap();

        config.set_audio_out_device("/dev/audio-out".to_string()).unwrap();
        config.set_rig_out_device("/dev/rig-out".to_string()).unwrap();
        config.set_rig_in_device("/dev/rig-in".to_string()).unwrap();

        assert_that!(config.get_keyer_type(), eq(KeyerType::Arduino));
        assert_that!(config.get_port(), eq("/dev/imaginary-usb-port"));
        assert_that!(config.get_wpm(), eq(40));

        assert_that!(config.get_audio_out_device(), eq("/dev/audio-out"));
        assert_that!(config.get_rig_out_device(), eq("/dev/rig-out"));
        assert_that!(config.get_rig_in_device(), eq("/dev/rig-in"));
        let reread_config = ConfigurationStore::new(temp.clone()).unwrap();

        assert_that!(reread_config.get_keyer_type(), eq(KeyerType::Arduino));
        assert_that!(reread_config.get_port(), eq("/dev/imaginary-usb-port"));
        assert_that!(reread_config.get_wpm(), eq(40));

        assert_that!(reread_config.get_audio_out_device(), eq("/dev/audio-out"));
        assert_that!(reread_config.get_rig_out_device(), eq("/dev/rig-out"));
        assert_that!(reread_config.get_rig_in_device(), eq("/dev/rig-in"));
    }
}
