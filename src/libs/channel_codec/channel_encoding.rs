use std::fmt::{Display, Formatter};
use std::fmt;

pub type ChannelSymbol = u8;

pub const CHANNEL_ENCODER_BLOCK_SIZE: usize = 64;

#[derive(Clone, PartialEq, Debug)]
pub struct ChannelEncoding {
    // Bytes of a block - containing the source encoded data, 2 spare bits, 14 bit CRC, and LDPC.
    // Maybe interleaved to ensure burst errors don't impact more critical areas of the data?
    // Gray encoded, and mapped to 4-bit symbols.
    // Prefixed with Costas Array symbols. The modulation code will add ramp up/down as necessary:
    // these will be ramped versions of the first and last symbols.
    pub block: Vec<ChannelSymbol>,
    // Is this encoding block the last in the sequence?
    pub is_end: bool,
}

impl Display for ChannelEncoding {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = if self.is_end { 'Y' } else { 'N' };
        write!(f, "End? {} Data [", c)?;
        for b in &self.block {
            write!(f, "{:02X?} ", b)?;
        }
        write!(f, "]")
    }
}
