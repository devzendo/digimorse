extern crate hamcrest2;

#[cfg(test)]
mod ldpc_spec {
    use std::env;

    use hamcrest2::prelude::*;
    use log::info;
    use substring::Substring;

    use crate::libs::channel_codec::crc::{CRC, crc14};
    use crate::libs::channel_codec::ldpc::{decode_codeword, encode_packed_message, init_ldpc, pack_message, unpack_message};
    use crate::libs::source_codec::source_encoding::{Frame, SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};
    use crate::libs::source_codec::test_encoding_builder::encoded;
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
