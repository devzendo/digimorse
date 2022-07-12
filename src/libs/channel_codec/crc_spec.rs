extern crate hamcrest2;

#[cfg(test)]
mod crc_spec {
    use std::env;
    use hamcrest2::prelude::*;
    use log::debug;
    use pretty_hex::*;
    use crate::libs::channel_codec::crc;
    use crate::libs::channel_codec::crc::{CRC, crc14, crc14_correct, crc14_slow};
    use crate::libs::util::util::vec_to_array;

    // 68 bytes of test data
    const INPUT: &[u8] = "This is a test of the emergency broadcast system. Do not be alarmed!".as_bytes();
    const INPUT_CRC: CRC = 0x06A9; // The CRC (verified) of INPUT.
    // Verified with the custom CRC Calculator at
    // https://ninja-calc.mbedded.ninja/calculators/software/crc-calculator

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    fn display_table() {
        for dividend in 0 .. 256 {
            debug!("CRC[{:>3}]=0x{:04X?}", dividend, crc::CRC_TABLE[dividend]);
        }
    }

    #[test]
    fn initial_table_values() {
        display_table();
        assert_that!(crc::CRC_TABLE[0],   equal_to(0x0000));
        assert_that!(crc::CRC_TABLE[1],   equal_to(0x6757));
        assert_that!(crc::CRC_TABLE[2],   equal_to(0xe9f9));
        assert_that!(crc::CRC_TABLE[3],   equal_to(0x8eae));
        assert_that!(crc::CRC_TABLE[4],   equal_to(0xf4a5));
        // ...
        assert_that!(crc::CRC_TABLE[255], equal_to(0xe899));
    }

    #[test]
    fn check_a_string() {
        let crc = crc14(INPUT);
        debug!("CRC=0x{:04X?}", crc);
        assert_that!(crc, equal_to(INPUT_CRC));

        let crc_slow = crc14_slow(INPUT);
        debug!("CRC_SLOW=0x{:04X?}", crc_slow);
        assert_that!(crc_slow, equal_to(INPUT_CRC))
    }

    #[test]
    fn verify_crc_passes() {
        let mut input_with_crc = Vec::from(INPUT);
        let shifted_crc = INPUT_CRC << 2; // Pack the 14 bits of CRC at the end of the INPUT.
        debug!("SHIFTED_CRC=0x{:04X?}", shifted_crc);
        input_with_crc.push((shifted_crc >> 8) as u8);
        input_with_crc.push(shifted_crc as u8);
        let input_with_crc_bytes = vec_to_array::<u8, 70>(input_with_crc);
        let hexdump = pretty_hex(&input_with_crc_bytes);
        debug!("data: {}", hexdump);

        assert_that!(crc14_correct(&input_with_crc_bytes), equal_to(true));
    }

    #[test]
    fn verify_crc_fails() {
        let mut input_with_crc = Vec::from(INPUT);
        input_with_crc[5] = input_with_crc[5] ^ 0x02; // flip one bit
        let shifted_crc = INPUT_CRC << 2; // Pack the 14 bits of CRC at the end of the INPUT.
        debug!("SHIFTED_CRC=0x{:04X?}", shifted_crc);
        input_with_crc.push((shifted_crc >> 8) as u8);
        input_with_crc.push(shifted_crc as u8);
        let input_with_crc_bytes = vec_to_array::<u8, 70>(input_with_crc);
        let hexdump = pretty_hex(&input_with_crc_bytes);
        debug!("data: {}", hexdump);

        assert_that!(crc14_correct(&input_with_crc_bytes), equal_to(false));
    }
}
