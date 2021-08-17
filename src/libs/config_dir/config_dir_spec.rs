extern crate hamcrest2;

#[cfg(test)]
mod config_dir_spec {
    use std::env;
    use hamcrest2::prelude::*;
    use temp_testdir::TempDir;
    use crate::libs::config_dir::config_dir::configuration_directory;
    use std::ops::Deref;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn config_dir_is_created() {
        let temp = TempDir::default();
        let result = configuration_directory(Some(temp.to_path_buf()));
        assert_that!(&result, ok());
        let config_dir_path = result.unwrap();
        let config_dir = config_dir_path.deref();
        // it's pretty much a random path, but it should have 'digimorse' in it somewhere near the
        // end...
        assert_that!(config_dir, path_exists());
        assert_that!(config_dir, dir_exists());
        let config_dir_str = config_dir.to_str().unwrap();
        assert_that!(config_dir_str, matches_regex("digimorse"));
    }
}
