#![feature(const_fn)]
#![feature(const_generics)]

use log::debug;

pub const POLYNOMIAL: u16 = 0x2757;
pub const WIDTH: u16 = 14; // Number of 8-bits that a CRC fits into; sizeof(u8) * 8
pub const TOPBIT: u16 = 1 << (WIDTH - 1);

// Generating static arrays during compile time, thanks to
// https://dev.to/rustyoctopus/generating-static-arrays-during-compile-time-in-rust-10d8

pub static CRC_TABLE: [u16; 256] = generate_table::<256>();

// CRC implementation using tables based on the article
// https://barrgroup.com/Embedded-Systems/How-To/CRC-Calculation-C-Code
// Using the same 14-bit polynomial as used by WSJT-X for FT8.

const fn generate_table<const DIM: usize>() -> [u16; DIM] {
    // This looks clunky; const fns can't use for, only while.
    let mut table: [u16; DIM] = [0; DIM];
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
                remainder = (remainder << 1);
            }
            bit -= 1;
        }
        table[dividend as usize] = remainder;
        dividend += 1;
    }
    table
}

fn display_table() {
    for dividend in 0 .. 256 {
        debug!("CRC[{:>3}]=0x{:04X?}", dividend, CRC_TABLE[dividend]);
    }
}

#[cfg(test)]
#[path = "./crc_spec.rs"]
mod crc_spec;
