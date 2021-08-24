extern crate hamcrest2;

#[cfg(test)]
mod config_file_spec {
    use log::{info};
    use std::env;
    use temp_testdir::TempDir;
    use crate::libs::config_file::config_file::ConfigurationStore;
    use hamcrest2::prelude::*;


    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn new_config_creates_a_toml_file() {
        let temp_dir = TempDir::default();
        let temp = temp_dir.to_path_buf();
        assert_that!(temp.as_path(), dir_exists());

        let config = ConfigurationStore::new(temp.into_boxed_path()).unwrap();

        let config_file_path = config.get_config_file_path();
        assert_that!(config_file_path, path_exists());
        assert_that!(config_file_path, file_exists());
        assert_that!(config_file_path.to_string_lossy(), matches_regex("digimorse.toml$"));
    }

    // TODO change wpm, save, reload to new object, assert wpm persisted round-trip
}
