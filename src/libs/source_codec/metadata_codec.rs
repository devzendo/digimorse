use crate::libs::source_codec::source_encoding::{Callsign, Locator, Power};

pub fn encode_callsign(_callsign: Callsign) -> u32 {
    0 // TODO
}

pub fn decode_callsign(_last_28_bits_of_encoded_callsign: u32) -> Callsign {
    "M0CUV".to_string() as Callsign // TODO
}

pub fn encode_locator(_locator: Locator) -> u16 {
    0
}

pub fn decode_locator(_last_15_bits_of_encoded_locator: u16) -> Locator {
    "JO01".to_string() as Locator // TODO
}

pub fn encode_power(_power: Power) -> u8 {
    0
}

pub fn decode_power(_last_n_bits_of_u8: u8) -> Power {
    0 as Power
}