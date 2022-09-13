extern crate hamcrest2;

#[cfg(test)]
mod ldpc_spec {
    use hamcrest2::prelude::*;
    use std::env;
    use ldpc::codes::LinearCode;
    use log::info;
    use sparse_bin_mat::{SparseBinMat, SparseBinSlice, SparseBinVec};
    use crate::libs::channel_codec::crc::crc14;
    use crate::libs::channel_codec::ldpc::{ColumnAccess, encode_message_to_sparsebinvec, init_ldpc, JohnsonFlipDecoder, LocalFlipDecoder};
    use crate::libs::channel_codec::ldpc_util::{display_matrix, draw_tanner_graph, generate_rust_for_matrix, load_parity_check_matrix, PARITY_CHECK_MATRIX_ALIST, PARITY_CHECK_MATRIX_RS, sparsebinvec_to_display};
    use crate::libs::channel_codec::parity_check_matrix::LDPC;
    use crate::libs::source_codec::source_encoding::{Frame, SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};
    use crate::libs::source_codec::test_encoding_builder::encoded;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn run_init_ldpc() {
        init_ldpc();
    }

    #[test]
    #[ignore]
    fn load_alist_into_sparsebinmat() {
        let sm = load_parity_check_matrix();
        info!("{}", sm.unwrap().as_json().unwrap());
    }

    // From "Iterative Error Correction", Example 2.5 "A regular parity-check matrix, with
    // Wc = 2 and Wr = 3"
    fn example_2_5_parity_check_matrix() -> SparseBinMat {
        SparseBinMat::new(6, vec![
            vec![0, 1, 3],
            vec![1, 2, 4],
            vec![0, 4, 5],
            vec![2, 3, 5],
        ])
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
    fn permute_example_2_5() {
        let ex2_5 = example_2_5_parity_check_matrix();
        let ex2_5_echelon = ex2_5.echelon_form();
        let ex2_5_display = display_matrix(&ex2_5_echelon);
        info!("parity check");
        for line in ex2_5_display.iter() {
            info!("{}", line);
        }
        let ldpc = LinearCode::from_parity_check_matrix(ex2_5_echelon);
        let g_display = display_matrix(ldpc.generator_matrix());
        info!("generator");
        for line in g_display.iter() {
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

    // Generate the rust code containing the parity check matrix that's been constructed via the
    // techniques described at the top of ldpc.rs.
    #[test]
    #[ignore]
    fn generate_rust_for_parity_check_matrix() {
        let sm = load_parity_check_matrix();
        assert_that!(generate_rust_for_matrix(&sm.unwrap(), PARITY_CHECK_MATRIX_ALIST, PARITY_CHECK_MATRIX_RS).is_ok(), true);
    }

    #[test]
    fn generated_parity_check_matrix_dimensions() {
        init_ldpc();
        let pcm = LDPC.parity_check_matrix();
        assert_that!(pcm.number_of_rows(), equal_to(126));
        assert_that!(pcm.number_of_columns(), equal_to(252));
    }

    #[test]
    fn generated_generator_matrix_dimensions() {
        init_ldpc();
        // UNKNOWN: Why does this need to be transposed to get the expected dimensions?
        let gen = LDPC.generator_matrix().transposed();
        assert_that!(gen.number_of_rows(), equal_to(252));
        assert_that!(gen.number_of_columns(), equal_to(126));
    }

    #[test]
    fn parity_times_generator_transpose_is_zero() {
        init_ldpc();
        let gen_t = LDPC.generator_matrix().transposed();
        let par = LDPC.parity_check_matrix();
        let mult = par * &gen_t;
        info!("mult is ({}, {})", mult.number_of_rows(), mult.number_of_columns()); // (126, 126)
        assert_that!(mult.is_zero(), equal_to(true));
    }

    fn generate_message() -> (SparseBinVec, String) {
        let source_encoding = encoded(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::WPMPolarity { wpm: 20, polarity: true },
            Frame::KeyingPerfectDit,
            Frame::KeyingPerfectDah,
            Frame::KeyingPerfectWordgap,
            Frame::KeyingDeltaDit { delta: 5 },
            Frame::KeyingDeltaDah { delta: -5 },
            Frame::KeyingDeltaWordgap { delta: 5 },
        ]);

        let crc = crc14(source_encoding.as_slice());
        info!("CRC=0x{:04X?}", crc);
        let crc_binary = format!("{:#016b}", crc);
        info!("CRC={}", crc_binary);

        (encode_message_to_sparsebinvec(source_encoding.as_slice(), crc), crc_binary)
    }

    #[test]
    fn source_encoding_to_sparsebinmat() {
        let (message, crc_binary) = generate_message();
        let mut message_string = sparsebinvec_to_display(&message);
        info!("message {}", message_string);
        assert_that!(message_string.len(), equal_to(126));

        let crc_at_end = message_string.split_off(126-14);
        assert_that!(crc_at_end, equal_to(crc_binary.strip_prefix("0b").unwrap()));
    }

    #[test]
    fn round_trip() {
        init_ldpc();
        let (message, _) = generate_message();
        let message_string = sparsebinvec_to_display(&message);
        info!("message  {}", message_string);

        let gen_t = LDPC.generator_matrix().transposed();
        let codeword = &gen_t * &message;
        let codeword_string = sparsebinvec_to_display(&codeword);
        info!("codeword {}", codeword_string);
        assert_that!(codeword_string.len(), equal_to(252));

        // let decoder = BpDecoder::new(LDPC.parity_check_matrix(), Probability::new(0.0), 100);
        let decoder = LocalFlipDecoder::new();
        // let decoder = JohnsonFlipDecoder::new(20);
        // The actual decoded message is the last 126 bits of 'decoded_message'?
        let decoded_message = decoder.decode(&codeword);
        let decoded_message_string = sparsebinvec_to_display(&decoded_message).split_off(126);
        info!("message  {}", message_string);
        info!("decoded  {}", decoded_message_string);
        assert_that!(decoded_message_string.len(), equal_to(126));
        assert_that!(decoded_message_string, equal_to(message_string)); // BROKEN: 1-bit error
    }

    // From p56
    #[test]
    fn johnson_flip_decoder_example_2_21() {
        let ex2_5 = example_2_5_parity_check_matrix();
        let code = LinearCode::from_parity_check_matrix(ex2_5);

        let y = SparseBinVec::new(6, vec![1, 2, 4, 5]); // 011011
        let y_string = sparsebinvec_to_display(&y);
        info!("y        {}", y_string);
        let decoder = JohnsonFlipDecoder::new(2);
        let decoded_message = decoder.decode(&y, &code);
        let decoded_message_string = sparsebinvec_to_display(&decoded_message);
        info!("y        {}", y_string);
        info!("decoded  {}", decoded_message_string);
        assert_that!(decoded_message_string, equal_to("001011"));
    }

    #[test]
    fn column_access() {
        let ex2_5 = example_2_5_parity_check_matrix();
        let row = ex2_5.row(1).unwrap(); // 011010
        info!("B2={}", row); // 1, 2, 4 ie positions where there's a 1 in the row
        let expected_row = SparseBinVec::new(6, vec![1, 2, 4]);
        assert_that!(row, equal_to(expected_row.as_view()));
        let column = ex2_5.column(1).unwrap();
        info!("A2={}", column);
        let expected_column = SparseBinVec::new(4, vec![0, 1]);
        assert_that!(column, equal_to(expected_column.as_view()));
    }
}
