extern crate hamcrest2;

#[cfg(test)]
mod ldpc_spec {
    use std::env;

    use hamcrest2::prelude::*;
    use ldpc::codes::LinearCode;
    use log::info;
    use sparse_bin_mat::{SparseBinMat, SparseBinVec};
    use substring::Substring;

    use crate::libs::channel_codec::crc::{CRC, crc14};
    use crate::libs::channel_codec::ex_2_5::example_2_5_parity_check_matrix;
    use crate::libs::channel_codec::ldpc::{decode_codeword, encode_message_to_sparsebinvec, encode_packed_message, init_ldpc, JohnsonFlipDecoder, LocalFlipDecoder, pack_message, unpack_message};
    use crate::libs::channel_codec::ldpc_util::{display_numpy_matrix, generate_rust_for_matrix, generate_rust_for_ldpc_init, GENERATOR_MATRIX_TXT, LDPC_INIT_RS, load_generator_matrix_and_columns, load_parity_check_matrix, PARITY_CHECK_MATRIX_ALIST, PARITY_CHECK_MATRIX_RS, sparsebinvec_to_display, display_matrix, display_sagemath_matrix};
    use crate::libs::channel_codec::ldpc_init::LDPC;
    use crate::libs::source_codec::source_encoding::{Frame, SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};
    use crate::libs::source_codec::test_encoding_builder::encoded;
    use crate::libs::sparse_binary_matrix::ColumnAccess;
    use crate::libs::util::util::dump_byte_vec_as_binary_stream;

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

        // FAILS
    }

    // Generate the rust code containing the parity check matrix that's been constructed via the
    // techniques described at the top of ldpc.rs.
    #[test]
    #[ignore]
    fn generate_rust_for_parity_check_matrix() {
        let pcm = load_parity_check_matrix().unwrap();
        info!("parity check matrix is ({}, {})", pcm.number_of_rows(), pcm.number_of_columns());
        assert_that!(pcm.number_of_rows(), equal_to(126));
        assert_that!(pcm.number_of_columns(), equal_to(252));
        assert_that!(generate_rust_for_matrix(&pcm, PARITY_CHECK_MATRIX_ALIST, PARITY_CHECK_MATRIX_RS).is_ok(), true);
    }

    // Generate the rust code containing the parity check and generator matrices that have been
    // constructed via the techniques described at the top of ldpc.rs.
    #[test]
    #[ignore]
    fn generate_rust_for_parity_check_and_generator_matrices() {
        let pcm = load_parity_check_matrix().unwrap();
        info!("parity check matrix is ({}, {})", pcm.number_of_rows(), pcm.number_of_columns());
        assert_that!(pcm.number_of_rows(), equal_to(126));
        assert_that!(pcm.number_of_columns(), equal_to(252));

        let (gm, cols) = load_generator_matrix_and_columns().unwrap();
        assert_that!(gm.number_of_rows(), equal_to(126));
        assert_that!(gm.number_of_columns(), equal_to(126));
        assert_that!(cols.len(), equal_to(252));

        let reordered_pcm = pcm.permute_columns(&cols.as_slice());

        // Pre/Suffix the generator with an Identity matrix to make it systematic. (as per Prof Johnson)
        let i126 = SparseBinMat::identity(126);
        let systematic_gm = gm.horizontal_concat_with(&i126); // suffix (Johnson)
        // let systematic_gm = i126.horizontal_concat_with(&gm); // prefix (Neal)
        assert_that!(systematic_gm.number_of_rows(), equal_to(126));
        assert_that!(systematic_gm.number_of_columns(), equal_to(252));
        display_matrix(&systematic_gm).iter().for_each(|f| info!("systematic generator {}", f));

        // Parity * generator(transposed) is zero (Neal)
        // let gm_t = systematic_gm.transposed();
        // let mult = &reordered_pcm * &gm_t;

        // Generator * parity(transposed) is zero (Johnson)
        let pcm_t = pcm.transposed();
        let mult = &systematic_gm * &pcm_t;

        info!("mult is ({}, {})", mult.number_of_rows(), mult.number_of_columns()); // (126, 126)
        display_matrix(&mult).iter().for_each(|f| info!("mult {}", f));
        assert_that!(mult.is_zero(), equal_to(true));


        assert_that!(generate_rust_for_ldpc_init(&reordered_pcm, &systematic_gm, &cols, PARITY_CHECK_MATRIX_ALIST, GENERATOR_MATRIX_TXT, LDPC_INIT_RS).is_ok(), true);
    }


    #[test]
    #[ignore]
    fn generate_sagemath_for_parity_check_matrix() {
        let pcm = load_parity_check_matrix().unwrap();

        let g_display = display_sagemath_matrix(&pcm, "H");
        for line in g_display.iter() {
            println!("{}", line);
        }
        // I pasted the sage definition of the parity check matrix printed above into sage, then:
        // print(H.rref())
        // However the output of this shows almost a perfect identity matrix on the left hand side
        // but the lowest 7 rows' 1s are shifted right, not on the diagonal.
        // Does this mean that to extract the message from a decoded codeword, I can't take exactly
        // the first 126 bits, but must take 119 then odd bits from the rest of the message - ie
        // do the columns that the leftmost 1s occur indicate the position of the message bits?
        // Not sure - the troublesome part of the matrix looks like this:
        //  0 1 0 0 0 0 0 0 1 0 0 0 1 0 0 0 0 0 0 1 1 0 0 1 0 1 1 0 0 0 0
        //  0 0 1 0 0 0 0 0 1 0 0 0 0 0 0 1 0 0 1 1 1 1 0 1 0 1 0 0 1 0 1
        //  0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0
        //  0 0 0 0 1 0 0 0 1 0 0 0 0 0 1 1 0 0 0 0 1 1 1 0 0 1 1 1 0 0 0
        //  0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 1 1 0 1 0 1 1 0 1
        //  0 0 0 0 0 0 1 0 0 0 0 0 0 0 1 1 0 0 0 0 1 0 1 0 0 1 1 1 0 0 1
        //  0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0
        //  0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 1 0 0 1 0 0 1 0 1 1 1 1 0 1 0 1
        //  0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 1 1 1 0 0 0 1 1 0 0
        //  0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 1 0 1 0 1 0 0 1 0 1 1 0 0
        //  0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0
        //  0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 1 0 1 0 0 0 0 0 0
        //  0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0
        //  0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0

    }


    #[test]
    #[ignore]
    fn parity_check_matrix_meets_density_requirements() {
        let pcm = load_parity_check_matrix().unwrap();
        for r in 0..pcm.number_of_rows() {
            let row = pcm.row(r);
            let weight = row.unwrap().weight();
            info!("row {} has weight {}", r, weight);
            // the row weights vary. is that right?
            //assert_that!(weight, equal_to(3));
        }
        for c in 0..pcm.number_of_columns() {
            let col = pcm.column(c);
            let weight = col.unwrap().weight();
            info!("col {} has weight {}", c, weight);
            assert_that!(weight, equal_to(3));
        }
    }

    #[test]
    fn generated_parity_check_matrix_dimensions() {
        init_ldpc();
        let pcm = LDPC.parity_check_matrix();
        assert_that!(pcm.number_of_rows(), equal_to(126));
        assert_that!(pcm.number_of_columns(), equal_to(252));

        // FAILS
    }

    #[test]
    fn generated_generator_matrix_dimensions() {
        init_ldpc();
        // UNKNOWN: Why does this need to be transposed to get the expected dimensions?
        let gen = LDPC.generator_matrix().transposed();
        assert_that!(gen.number_of_rows(), equal_to(252));
        assert_that!(gen.number_of_columns(), equal_to(126));

        // FAILS
    }

    #[test]
    fn parity_times_generator_transpose_is_zero() {
        init_ldpc();
        let gen_t = LDPC.generator_matrix().transposed();
        let par = LDPC.parity_check_matrix();
        let mult = par * &gen_t;
        info!("mult is ({}, {})", mult.number_of_rows(), mult.number_of_columns()); // (126, 126)
        assert_that!(mult.is_zero(), equal_to(true));

        // FAILS
    }

    #[test]
    #[ignore]
    fn display_transposed_generator_matrix() {
        init_ldpc();
        let gen_t = LDPC.generator_matrix().transposed();
        let g_display = display_numpy_matrix(&gen_t, "G");
        for line in g_display.iter() {
            println!("{}", line);
        }
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

        let decoded_message = decoder.decode(&codeword);
        // The actual decoded message is the last 126 bits of 'decoded_message'?
        let decoded_message_string = sparsebinvec_to_display(&decoded_message).split_off(126);
        info!("message  {}", message_string);
        info!("decoded  {}", decoded_message_string);
        assert_that!(decoded_message_string.len(), equal_to(126));
        assert_that!(decoded_message, equal_to(codeword));
        assert_that!(decoded_message_string, equal_to(message_string)); // BROKEN: 1-bit error

        // FAILS
    }

    #[test]
    fn round_trip_johnson_flip_decoder() {
        init_ldpc();
        let (message, _) = generate_message();
        info!("message      {}", message);
        let message_string = sparsebinvec_to_display(&message);
        info!("message_str  {}", message_string);

        let gen_t = LDPC.generator_matrix().transposed();
        let codeword = &gen_t * &message;
        let codeword_string = sparsebinvec_to_display(&codeword);
        info!("codeword_str {}", codeword_string);
        assert_that!(codeword_string.len(), equal_to(252));

        let decoder = JohnsonFlipDecoder::new(20);
        let decoded_codeword = decoder.decode(&codeword, &LDPC);
        assert_that!(decoded_codeword.clone(), equal_to(codeword));

        // To get the decoded message out of the decoded codeword, multiply the decoded codeword by
        // the parity check matrix. This was not obvious to me; not mentioned in "Iterative Error
        // Correction" or other introductions; It was however made clear in..
        // (p.28 of https://core.ac.uk/download/pdf/37320505.pdf
        // An LDPC Error Control Strategy for Low Earth Orbit
        // Satellite Communication Link Applications
        // F.J. Olivier )
        let par = LDPC.parity_check_matrix().transposed();
        let decoded_codeword_matrix = SparseBinMat::new(decoded_codeword.len(), vec![decoded_codeword.to_positions_vec()]);
        let decoded_message =  &decoded_codeword_matrix * &par;
        assert_that!(decoded_message.number_of_rows(), equal_to(1));
        assert_that!(decoded_message.number_of_columns(), equal_to(126));
        let decoded_message_string = sparsebinvec_to_display(&decoded_message.row(0).unwrap().to_vec());

        info!("message_str          {}", message_string);
        info!("codeword_str         {}", codeword_string);
        info!("decoded_message_str  {}", decoded_message_string);
        assert_that!(decoded_message_string.len(), equal_to(126));

        assert_that!(decoded_message_string, equal_to(message_string)); // BROKEN: decoded_message is all zeros

        // FAILS
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
        let column_vec = ex2_5.column(1).unwrap();
        let column_slice = column_vec.as_view();
        info!("A2={}", column_slice);
        let expected_column = SparseBinVec::new(4, vec![0, 1]);
        assert_that!(column_slice, equal_to(expected_column.as_view()));
    }


    fn generate_message_and_crc() -> (Vec<u8>, CRC, String) {
        let source_encoding = encoded(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::WPMPolarity { wpm: 20, polarity: true },
            Frame::KeyingPerfectDit,
            Frame::KeyingPerfectDah,
            Frame::KeyingPerfectWordgap,
            Frame::KeyingDeltaDit { delta: 5 },
            Frame::KeyingDeltaDah { delta: -5 },
            Frame::KeyingDeltaWordgap { delta: 5 },
        ]);
        let encoding_binary = dump_byte_vec_as_binary_stream(&source_encoding);
        info!("Source encoded message: {}", encoding_binary);

        let crc = crc14(source_encoding.as_slice());
        info!("CRC=0x{:04X?}", crc);
        let crc_binary = format!("{:#016b}", crc);
        info!("CRC={}", crc_binary);

        (source_encoding, crc, crc_binary)
    }

    #[test]
    fn source_encoding_to_packed_message() {
        let (message, crc, crc_binary) = generate_message_and_crc();
        let message_string = dump_byte_vec_as_binary_stream(&message);

        let packed_message = pack_message(&message, false, true, crc);
        let mut packed_message_string = dump_byte_vec_as_binary_stream(&packed_message);
        info!("packed message          {}", packed_message_string);
        assert_that!(packed_message_string.len(), equal_to(128));

        let message_at_start = packed_message_string.substring(0, 112);
        assert_that!(message_at_start, equal_to(message_string.as_str()));

        let unused_flag_1 = packed_message_string.substring(112, 113);
        assert_that!(unused_flag_1, equal_to("0"));

        let unused_flag_2 = packed_message_string.substring(113, 114);
        assert_that!(unused_flag_2, equal_to("1"));

        let crc_at_end = packed_message_string.split_off(128-14);
        assert_that!(crc_at_end, equal_to(crc_binary.strip_prefix("0b").unwrap()));
    }

    #[test]
    fn packed_message_to_encoded_packed_message() {
        let (message, crc, _) = generate_message_and_crc();

        let packed_message = pack_message(&message, false, true, crc);
        let packed_message_string = dump_byte_vec_as_binary_stream(&packed_message);
        info!("packed message:         {}", packed_message_string);

        let encoded_packed_message = encode_packed_message(&packed_message);
        let encoded_packed_message_string = dump_byte_vec_as_binary_stream(&encoded_packed_message);
        info!("encoded packed message: {}", encoded_packed_message_string);

        let packed_message_at_start = encoded_packed_message_string.substring(0, 128);
        assert_that!(packed_message_at_start, equal_to(packed_message_string));
    }

    #[test]
    fn round_trip_no_corruption() {
        let (message, crc, _) = generate_message_and_crc();
        let packed_message = pack_message(&message, false, true, crc);
        let packed_message_string = dump_byte_vec_as_binary_stream(&packed_message);
        let encoded_packed_message = encode_packed_message(&packed_message);
        let encoded_packed_message_string = dump_byte_vec_as_binary_stream(&encoded_packed_message);
        info!("encoded packed message: {}", encoded_packed_message_string);

        // no corruption

        let decoded_packed_message = decode_codeword(&encoded_packed_message).unwrap();
        let decoded_packed_message_string = dump_byte_vec_as_binary_stream(&decoded_packed_message);
        info!("decoded packed message: {}", decoded_packed_message_string);

        let packed_message_at_start = decoded_packed_message_string.substring(0, 128);
        assert_that!(packed_message_at_start, equal_to(packed_message_string));
    }

    #[test]
    fn round_trip_with_corruption() {
        let (message, crc, _) = generate_message_and_crc();
        let packed_message = pack_message(&message, false, true, crc);
        let packed_message_string = dump_byte_vec_as_binary_stream(&packed_message);
        let mut encoded_packed_message = encode_packed_message(&packed_message);
        let encoded_packed_message_string = dump_byte_vec_as_binary_stream(&encoded_packed_message);
        info!("encoded packed message (original): {}", encoded_packed_message_string);
        info!("length {}", encoded_packed_message.len());

        // corrupt the codeword
        // corruptions | iterations for decode
        // 15          | fails
        // 14          | 6
        // 13          | 5
        // 12          | 3
        // 11          | 3
        // 10          | 3
        // 9           | 3
        // 8           | 2
        // 7           | 4 (oddly non-monotonic?)
        // 6           | 4
        // 5           | 3
        // 4           | 1
        // 3           | 1
        // 2           | 1
        // 1           | 1
        // 0           | 0
        let num_corruptions = 14;
        for i in 0..num_corruptions {
            encoded_packed_message[i] ^= 0x10;
        }

        let corrupt_encoded_packed_message_string = dump_byte_vec_as_binary_stream(&encoded_packed_message);
        info!("encoded packed message (corrupt):  {}", corrupt_encoded_packed_message_string);

        let decoded_packed_message = decode_codeword(&encoded_packed_message).unwrap();
        let decoded_packed_message_string = dump_byte_vec_as_binary_stream(&decoded_packed_message);
        info!("decoded packed message:            {}", decoded_packed_message_string);

        let packed_message_at_start = decoded_packed_message_string.substring(0, 128);
        assert_that!(packed_message_at_start, equal_to(packed_message_string));
    }

    #[test]
    fn unpack_packed_message() {
        let (message, crc, _) = generate_message_and_crc();

        let packed_message = pack_message(&message, false, true, crc);

        let (u_message, u_unused_flag_1, u_unused_flag_2, u_crc) = unpack_message(&packed_message);
        assert_that!(u_message, equal_to(message));
        assert_that!(u_unused_flag_1, equal_to(false));
        assert_that!(u_unused_flag_2, equal_to(true));
        assert_that!(u_crc, equal_to(crc));
    }
}
