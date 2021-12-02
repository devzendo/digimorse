use std::error::Error;
use crate::libs::source_codec::source_encoding::Frame;

/// Decode an encoded block. This is either error-free or has been extracted from an error-corrected
/// block; i.e. no decode errors should occur. Just in case there's a decode error, this returns a
/// Result.
pub fn source_decode(encoded_block: Vec<u8>) -> Result<Vec<Frame>, Box<dyn Error>> {
    let frames: Vec<Frame> = vec![];
    Ok(frames)
}



#[cfg(test)]
#[path = "./source_decoder_spec.rs"]
mod source_decoder_spec;
