
extern crate lazy_static;
use lazy_static::lazy_static;

lazy_static! {                            // 0, 1, 2, 3, 4, 5, 6, 7, 8,  9,  10, 11, 12, 13, 14, 15
  pub static ref BINARY_TO_GRAY: [u8; 16] = [0, 1, 3, 2, 6, 7, 5, 4, 12, 13, 15, 14, 10, 11, 9,  8];
  pub static ref GRAY_TO_BINARY: [u8; 16] = [0, 1, 3, 2, 7, 6, 4, 5, 15, 14, 12, 13, 8,  9,  11, 10 ];
}

// nybble only has the least significant 4 bits set; only these bits are returned
pub fn to_gray_code(nybble: u8) -> u8 {
    if nybble > 0x0f {
        panic!("to_gray_code received a {} which is > 0x0f", nybble)
    }
    BINARY_TO_GRAY[nybble as usize]
}

// gray_nybble only has the least significant 4 bits set; only these bits are returned
pub fn from_gray_code(gray_nybble: u8) -> u8 {
    if gray_nybble > 0x0f {
        panic!("from_gray_code received a {} which is > 0x0f", gray_nybble)
    }
    GRAY_TO_BINARY[gray_nybble as usize]
}

#[cfg(test)]
#[path = "gray_spec.rs"]
mod gray_spec;