extern crate hamcrest2;

#[cfg(test)]
mod ldpc_util_spec {
    use std::{env, fs};

    use hamcrest2::prelude::*;
    use log::info;

    use crate::libs::channel_codec::ldpc_util::{display_matrix, from_alist};

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn ex_2_5_parity_check_matrix_from_alist() {
        let ex2_5 = from_alist(fs::read_to_string("src/libs/channel_codec/ex_2_5_parity_check_matrix.alist").unwrap().as_str()).unwrap();
        let ex2_5_display = display_matrix(&ex2_5);
        for line in ex2_5_display.iter() {
            info!("{}", line);
        }
        assert_that!(ex2_5_display[0].as_str(), equal_to("1 1 0 1 0 0 "));
        assert_that!(ex2_5_display[1].as_str(), equal_to("0 1 1 0 1 0 "));
        assert_that!(ex2_5_display[2].as_str(), equal_to("1 0 0 0 1 1 "));
        assert_that!(ex2_5_display[3].as_str(), equal_to("0 0 1 1 0 1 "));
    }
}
