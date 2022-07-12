use std::convert::TryInto;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn printable(ch: u8) -> String {
    let chr = ch as char;
    return format!("0x{:02x} {}", ch, if chr.is_ascii_alphanumeric() || chr.is_ascii_punctuation() { chr } else { '.' });
}

pub fn dump_byte_vec(bytes: &Vec<u8>) -> String {
    let mut out = vec![];
    for b in bytes {
        out.push(format!("{:#010b}", b));
    }
    format!("[{}]", out.join(", "))
}

// From https://stackoverflow.com/questions/29570607/is-there-a-good-way-to-convert-a-vect-to-an-array
pub fn vec_to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

pub fn get_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

