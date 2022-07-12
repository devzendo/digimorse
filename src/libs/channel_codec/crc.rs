use log::debug;

pub type CRC = u16;

pub const POLYNOMIAL: u16 = 0x2757;
pub const WIDTH: u16 = 14; // Number of 8-bits that a CRC fits into; sizeof(u8) * 8
pub const TOPBIT: u16 = 1 << (WIDTH - 1);

// Generating static arrays during compile time, thanks to
// https://dev.to/rustyoctopus/generating-static-arrays-during-compile-time-in-rust-10d8

pub static CRC_TABLE: [CRC; 256] = generate_table::<256>();

// CRC implementation using tables based on the article
// https://barrgroup.com/Embedded-Systems/How-To/CRC-Calculation-C-Code
// Using the same 14-bit polynomial as used by WSJT-X for FT8.

const fn generate_table<const DIM: usize>() -> [CRC; DIM] {
    // This looks clunky; const fns can't use for, only while.
    let mut table: [CRC; DIM] = [0; DIM];
    // Compute the remainder of each dividend
    let mut dividend = 0;
    while dividend < 256 {
        // Start with dividend followed by zeros
        let mut remainder: u16 = dividend << (WIDTH - 8);
        // Perform modulo-2 division, a bit at a time
        let mut bit = 8;
        while bit > 0 {
            // Try to divide the current data bit.
            if remainder & TOPBIT != 0 {
                remainder = (remainder << 1) ^ POLYNOMIAL;
            }
            else {
                remainder = remainder << 1;
            }
            bit -= 1;
        }
        table[dividend as usize] = remainder as CRC;
        dividend += 1;
    }
    table
}

fn display_table() {
    for dividend in 0 .. 256 {
        debug!("CRC[{:>3}]=0x{:04X?}", dividend, CRC_TABLE[dividend]);
    }
}

pub fn crc14(data: &[u8]) -> CRC {
    let mut remainder: CRC = 0;
    // Divide the data by the polynomial, a byte at a time.
    for i in 0 .. data.len() {
        let index = (data[i] ^ ((remainder >> (WIDTH - 8)) as u8)) as usize;
        // debug!("DATA[{}]=0x[{:02X}];  CRC[{:>3}]=0x{:04X?}", i, data[i], index, CRC_TABLE[index]);
        remainder = CRC_TABLE[index] ^ (remainder << 8);
        // debug!("REMAINDER=0x{:04X?}", remainder);
    }
    remainder & 0x3fff
}

pub fn crc14_correct(data_with_crc_appended: &[u8]) -> bool {
    // debug!("CRC14_CORRECT");
    let crc = crc14(data_with_crc_appended);
    // debug!("CORRECT CRC?=0x{:04X?}", crc);
    crc == 0x0000
}

// This variant is here for comparison with the above routines.
pub fn crc14_slow(data: &[u8]) -> CRC {
    let mut remainder: CRC = 0;
    // Perform modulo-2 division, a byte at a time.
    for byte in 0 .. data.len() {
        // Bring the next byte into the remainder.
        remainder ^= (data[byte] as u16) << (WIDTH - 8);
        // Perform modulo-2 division, a bit at a time.
        for bit in (0 .. 8).rev() {
            if (remainder & TOPBIT) != 0x0000 {
                remainder = (remainder << 1) ^ POLYNOMIAL;
            } else {
                remainder = remainder << 1;
            }
        }
    }
    remainder & 0x3fff
}

#[cfg(test)]
#[path = "./crc_spec.rs"]
mod crc_spec;
