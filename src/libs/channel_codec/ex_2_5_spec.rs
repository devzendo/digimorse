extern crate hamcrest2;

#[cfg(test)]
mod ex_2_5_spec {
    use std::{env, fs};

    use hamcrest2::prelude::*;
    use ldpc::codes::LinearCode;
    use log::info;
    use sparse_bin_mat::SparseBinMat;
    use crate::libs::channel_codec::ex_2_5::example_2_5_parity_check_matrix;
    use crate::libs::channel_codec::ldpc::JohnsonFlipDecoder;
    use crate::libs::channel_codec::ldpc_util::{from_alist, draw_tanner_graph, display_matrix};

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn use_ldpc_to_create_generator_given_ex_2_5_parity_check() {
        let ex2_5_pcm = example_2_5_parity_check_matrix();
        let ex2_5_pcm_clone = ex2_5_pcm.clone();
        let code = LinearCode::from_parity_check_matrix(ex2_5_pcm);
        let ex2_5_gm = code.generator_matrix();
        assert_that!(ex2_5_gm.number_of_rows(), equal_to(3));
        assert_that!(ex2_5_gm.number_of_columns(), equal_to(6));
        let ex2_5_gm_display = display_matrix(&ex2_5_gm);
        for line in ex2_5_gm_display.iter() {
            info!("{}", line);
        }
        assert_that!(ex2_5_gm_display[0].as_str(), equal_to("0 1 1 1 0 0 "));
        assert_that!(ex2_5_gm_display[1].as_str(), equal_to("1 1 0 0 1 0 "));
        assert_that!(ex2_5_gm_display[2].as_str(), equal_to("1 1 1 0 0 1 "));
        // check dot product is 0
        let ex2_5_gm_transposed = ex2_5_gm.transposed();
        let mult = &ex2_5_pcm_clone * &ex2_5_gm_transposed;
        assert_that!(mult.is_zero(), equal_to(true));
    }

    #[test]
    fn ex_2_5_round_trip() {
        let ex2_5_pcm = example_2_5_parity_check_matrix();
        let ex2_5_pcm_clone = ex2_5_pcm.clone();
        let code = LinearCode::from_parity_check_matrix(ex2_5_pcm);
        let ex2_5_gm = code.generator_matrix();

        let msg = SparseBinMat::new(3, vec![ vec![0, 2] ]); // 1 0 1
        let cw = &msg * ex2_5_gm;
        let cw_display = display_matrix(&cw);
        for line in cw_display.iter() {
            info!("{}", line);
        }
        assert_that!(&cw_display, len(1));
        assert_that!(cw_display.get(0).unwrap().as_str(), matches_regex("^. . . 1 0 1 $"));

        // Is this a valid codeword? H cw^T must be zero
        let cw_transposed = cw.transposed();
        let h_cw_t = &ex2_5_pcm_clone * &cw_transposed;
        assert_that!(h_cw_t.is_zero(), equal_to(true));

        // Decode
        let flip = JohnsonFlipDecoder::new(10);
        let cw_row = cw.row(0).unwrap().to_vec();
        let decoded = flip.decode(&cw_row, &code);
        let decoded_matrix = SparseBinMat::new(decoded.len(), vec![decoded.non_trivial_positions().into_iter().collect()]);
        let dm_display = display_matrix(&decoded_matrix);
        for line in dm_display.iter() {
            info!("{}", line);
        }
        assert_that!(&dm_display, len(1));
        assert_that!(dm_display.get(0).unwrap().as_str(), matches_regex("^. . . 1 0 1 $"));
        // The last 3 columns of the generator matrix are the identity matrix, so the last
        // 3 columns of this decoded codeword are the decoded message.
        // TODO assert that, and iterate over all 8 possible 3-bit input messages, asserting the
        // round trip works.
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
}
