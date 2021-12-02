extern crate num;

use std::fmt::{Display, Formatter};
use std::fmt;

use crate::libs::keyer_io::keyer_io::KeyerSpeed;

// Size of all source encoder frames; could change as the design of later stages evolves.
// TODO what is the ideal size of this? * WHAT DOES THE LDPC (CHANNEL ENCODER) REQUIRE AS ITS INPUT?
pub const SOURCE_ENCODER_BLOCK_SIZE_IN_BITS: usize = 64;
// multiple of 8?

#[derive(Clone, PartialEq)]
pub struct SourceEncoding {
    // bytes of a block
    pub block: Vec<u8>,
    // Is this encoding block the last in the sequence?
    pub is_end: bool,
}

impl Display for SourceEncoding {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = if self.is_end { 'Y' } else { 'N' };
        write!(f, "End? {} Data [", c)?;
        for b in &self.block {
            write!(f, "{:02X?} ", b)?;
        }
        write!(f, "]")
    }
}

// Multiple implementations (possibly) to find the fastest low-level bit vector crate, there are
// many.
pub trait SourceEncodingBuilder {
    /// Call size() before adding, if there's not enough space to store the data, call build() to
    /// get the SourceEncoding, and the storage will be reset for another block.
    fn size(&self) -> usize;
    /// As an alternative to size(), remaining() tells you how many more bits you could fit into
    /// this block.
    fn remaining(&self) -> usize;
    /// Add a number of bits from the right-hand side (least significant bits) of a u8.
    fn add_8_bits(&mut self, data: u8, num_bits: usize);
    /// Add a number of bits from the right-hand side (least significant bits) of a u16.
    fn add_16_bits(&mut self, data: u16, num_bits: usize);
    /// Add a number of bits from the right-hand side (least significant bits) of a u32.
    fn add_32_bits(&mut self, data: u32, num_bits: usize);
    /// Add a bit from a bool.
    fn add_bool(&mut self, data: bool);
    /// Set the 'end' state.
    fn set_end(&mut self);
    /// Build the SourceEncoding by padding it out to the block size, and reset the storage.
    fn build(&mut self) -> SourceEncoding;
}

pub type Callsign = String;
pub type CallsignHash = u16; // MAYBE?
pub type Locator = String;
pub type Power = u8; // MAYBE?
pub type KeyingDelta = i16;
pub type KeyingNaive = u16;

enum_from_primitive! {
#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub enum EncoderFrameType {
    Padding = 0,
    WPMPolarity,
    CallsignMetadata,
    CallsignHashMetadata,
    LocatorMetadata,
    PowerMetadata,
    KeyingPerfectDit,
    KeyingPerfectDah,
    KeyingPerfectWordgap,
    KeyingEnd,
    KeyingDeltaDit,
    KeyingDeltaDah,
    KeyingDeltaWordgap,
    KeyingNaive,
    Unused,
    Extension,
}
}

/// Decoded frames are of this type. It's also used to create encoded frames for test data.
pub enum Frame {
    Padding,
    WPMPolarity { wpm: KeyerSpeed, polarity: bool },
    CallsignMetadata { callsign: Callsign },
    CallsignHashMetadata { hash: CallsignHash },
    LocatorMetadata { locator: Locator },
    KeyingPerfectDit,
    KeyingPerfectDah,
    KeyingPerfectWordgap,
    KeyingEnd,
    KeyingDeltaDit { delta: KeyingDelta },
    KeyingDeltaDah { delta: KeyingDelta },
    KeyingDeltaWordgap { delta: KeyingDelta },
    KeyingNaive { duration: KeyingNaive },
    Unused,
    Extension,
}


#[cfg(test)]
#[path = "./source_encoding_spec.rs"]
mod source_encoding_spec;
