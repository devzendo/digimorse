extern crate hamcrest2;

#[cfg(test)]
mod config_file_spec {
    use std::env;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn get_new_config() {}
}
