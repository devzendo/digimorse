extern crate hamcrest2;

#[cfg(test)]
mod crc_spec {
    use std::env;
    use hamcrest2::prelude::*;
    use crate::libs::channel_codec::crc;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn initial_table_values() {
        crc::display_table();
        assert_that!(crc::CRC_TABLE[0],   equal_to(0x0000));
        assert_that!(crc::CRC_TABLE[1],   equal_to(0x6757));
        assert_that!(crc::CRC_TABLE[2],   equal_to(0xe9f9));
        assert_that!(crc::CRC_TABLE[3],   equal_to(0x8eae));
        assert_that!(crc::CRC_TABLE[4],   equal_to(0xf4a5));
        // ...
        assert_that!(crc::CRC_TABLE[255], equal_to(0xe899));
    }
}
