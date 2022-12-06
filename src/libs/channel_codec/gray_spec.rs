#[cfg(test)]
mod gray_spec {
    use std::env;
    use crate::libs::channel_codec::gray::{from_gray_code, to_gray_code};

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn round_trip() {
        for i in 0..16 {
            assert_eq!(from_gray_code(to_gray_code(i)), i);
        }
    }
}