extern crate hamcrest2;

#[cfg(test)]
mod ldpc_util_spec {
    use std::{env, fs};

    use hamcrest2::prelude::*;
    use log::info;
    use crate::libs::channel_codec::ex_2_5::example_2_5_parity_check_matrix;
    use crate::libs::channel_codec::ldpc_util::{from_alist, load_parity_check_matrix, draw_tanner_graph, display_matrix};

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    #[ignore]
    fn load_alist_into_sparsebinmat() {
        let pcm = load_parity_check_matrix().unwrap();
        info!("{}", pcm.as_json().unwrap());
        assert_that!(pcm.number_of_rows(), equal_to(126));
        assert_that!(pcm.number_of_columns(), equal_to(252));
    }

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

        assert_that!(ex2_5, equal_to(example_2_5_parity_check_matrix()));
    }

    #[test]
    #[ignore]
    fn draw_example_2_5_tanner_graph() {
        let ex2_5 = example_2_5_parity_check_matrix();
        assert_that!(draw_tanner_graph(&ex2_5, "/tmp/example_2_5.dot").is_ok(), true);
    }

    #[test]
    #[ignore]
    fn draw_example_2_5_as_matrix() {
        let ex2_5 = example_2_5_parity_check_matrix();
        let ex2_5_display = display_matrix(&ex2_5);
        for line in ex2_5_display.iter() {
            info!("{}", line);
        }
    }

    #[test]
    #[ignore]
    fn draw_parity_check_matrix_as_tanner_graph() {
        let sm = load_parity_check_matrix();
        assert_that!(draw_tanner_graph(&sm.unwrap(), "/tmp/digimorse_parity_check_matrix.dot").is_ok(), true);
        // dot -Tpng /tmp/digimorse_parity_check_matrix.dot -o /tmp/digimorse_parity_check_matrix.png
        // takes a few minutes to generate, complains about being too big, and scaling...
        // and is quite unreadable!
    }
}
