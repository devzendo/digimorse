extern crate hamcrest2;

#[cfg(test)]
mod ldpc_util_spec {
    use std::{env, fs};

    use hamcrest2::prelude::*;
    use log::info;
    use sparse_bin_mat::BinNum;
    use crate::libs::channel_codec::ldpc_util::{draw_tanner_graph, from_gen_txt, load_parity_check_matrix};

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
    #[ignore]
    fn draw_parity_check_matrix_as_tanner_graph() {
        let sm = load_parity_check_matrix();
        assert_that!(draw_tanner_graph(&sm.unwrap(), "/tmp/digimorse_parity_check_matrix.dot").is_ok(), true);
        // dot -Tpng /tmp/digimorse_parity_check_matrix.dot -o /tmp/digimorse_parity_check_matrix.png
        // takes a few minutes to generate, complains about being too big, and scaling...
        // and is quite unreadable!
    }

    #[test]
    #[ignore]
    fn load_generator_matrix() {
        let (g, cols) = from_gen_txt(fs::read_to_string("src/libs/channel_codec/generator_matrix.txt").unwrap().as_str()).unwrap();
        assert_that!(g.number_of_rows(), equal_to(126));
        assert_that!(g.number_of_columns(), equal_to(126));
        assert_that!(g.get(0, 0).unwrap(), equal_to(BinNum::zero()));
        assert_that!(g.get(125, 0).unwrap(), equal_to(BinNum::zero()));
        assert_that!(g.get(0, 125).unwrap(), equal_to(BinNum::zero()));
        assert_that!(g.get(125, 125).unwrap(), equal_to(BinNum::one()));
        assert_that!(cols.len(), equal_to(252));
        assert_that!(cols, equal_to(vec![
            78, 24, 79, 49, 1, 43, 62, 76, 19, 114, 22, 59, 108, 17, 91, 46, 61, 28, 65, 41,
            33, 60, 74, 63, 77, 26, 39, 21, 34, 50, 38, 27, 15, 14, 58, 35, 13, 18, 52, 103,
            5, 30, 11, 56, 29, 127, 57, 8, 54, 53, 32, 36, 42, 31, 140, 4, 71, 70, 16, 37,
            7, 44, 128, 69, 2, 82, 40, 72, 88, 3, 66, 73, 101, 118, 104, 80, 86, 97, 87, 47,
            83, 89, 64, 93, 90, 94, 23, 99, 68, 20, 75, 115, 95, 12, 92, 96, 85, 55, 98, 10,
            102, 100, 0, 105, 67, 107, 84, 109, 110, 25, 106, 111, 112, 113, 116, 81, 9, 123, 120, 51,
            119, 122, 121, 124, 117, 125, 126, 45, 6, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139,
            48, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159,
            160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179,
            180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199,
            200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219,
            220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239,
            240, 241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251
        ]));
    }
}
