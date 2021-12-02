use bitvec::prelude::*;
use std::error::Error;
use crate::libs::source_codec::source_encoding::{Frame, SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};

/// Decode an encoded block. This is either error-free or has been extracted from an error-corrected
/// block; i.e. no decode errors should occur. Just in case there's a decode error, this returns a
/// Result.
pub fn source_decode(encoded_block: Vec<u8>) -> Result<Vec<Frame>, Box<dyn Error>> {
    if encoded_block.len() != (SOURCE_ENCODER_BLOCK_SIZE_IN_BITS >> 3) {
        return Err(Box::<dyn Error + Send + Sync>::from(format!("Cannot decode a block of the wrong size")));
    }
    let frames: Vec<Frame> = vec![];
    let mut bit_vec = BitVec::<Msb0, u8>::with_capacity(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);

    Ok(frames)
}



#[cfg(test)]
#[path = "./source_decoder_spec.rs"]
mod source_decoder_spec;
