use log::{debug, info};
use std::error::Error;
use crate::enum_primitive::FromPrimitive;
use crate::libs::source_codec::bitvec_source_encoding_extractor::BitvecSourceEncodingExtractor;
use crate::libs::source_codec::keying_encoder::decode_from_binary_with_known_sign;
use crate::libs::source_codec::keying_timing::{DefaultKeyingTiming, KeyingTiming};
use crate::libs::source_codec::source_encoding::{EncoderFrameType, Frame, SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, SourceEncodingExtractor};
use crate::libs::util::util::dump_byte_vec;

/// Decode an encoded block. This is either error-free or has been extracted from an error-corrected
/// block; i.e. no decode errors should occur. Just in case there's a decode error, this returns a
/// Result (which may result in white noise being played, to indicate this to the user).
pub fn source_decode(encoded_block: Vec<u8>) -> Result<Vec<Frame>, Box<dyn Error>> {
    if encoded_block.len() != (SOURCE_ENCODER_BLOCK_SIZE_IN_BITS >> 3) {
        return Err(Box::<dyn Error + Send + Sync>::from(format!("Cannot decode a block of the wrong size")));
    }
    debug!("Decoding {}", dump_byte_vec(&encoded_block));
    let no_wpm_polarity_err = Err(Box::<dyn Error>::from("Cannot decode keying without prior WPM|Polarity"));
    let mut timing = DefaultKeyingTiming::new();
    let mut seen_wpm_polarity = false;
    let mut frames: Vec<Frame> = vec![];
    let mut extractor = BitvecSourceEncodingExtractor::new(encoded_block);
    loop {
        let remaining = extractor.remaining();
        if remaining < 4 {
            debug!("End of decode: Insufficient data ({} bits) to read field type", remaining);
            break;
        }
        let field_type_nibble= extractor.extract_8_bits(4);
        let maybe_field_type = EncoderFrameType::from_u8(field_type_nibble);
        match maybe_field_type {
            None => {
                // Cannot happen, since we've extracted 4 bits and they're all valid EncoderFrameTypes.
                panic!("Cannot interpret {:#010b} as an EncoderFrameType", field_type_nibble);
            }
            // Note: The source encoder ensures that frames are atomic, that is, you won't find a
            // frame header without its data. If the frame would not fit at the end of a block, it
            // would have been placed in the next block.
            Some(field_type) => {
                debug!("Decoding field {:?}", field_type);
                match field_type {
                    EncoderFrameType::Padding => {
                        frames.push(Frame::Padding);
                        // No need to decode further, padding indicates end of block.
                        debug!("End of decode: Padding");
                        break;
                    }
                    EncoderFrameType::WPMPolarity => {
                        seen_wpm_polarity = true;
                        let keying_speed = extractor.extract_8_bits(6);
                        let mark = extractor.extract_bool();
                        timing.set_keyer_speed(keying_speed);
                        frames.push(Frame::WPMPolarity { wpm: keying_speed, polarity: mark });
                    }
                    EncoderFrameType::CallsignMetadata => {
                        todo!();
                    }
                    EncoderFrameType::CallsignHashMetadata => {
                        todo!();
                    }
                    EncoderFrameType::LocatorMetadata => {
                        todo!();
                    }
                    EncoderFrameType::PowerMetadata => {
                        todo!();
                    }
                    EncoderFrameType::KeyingPerfectDit => {
                        if !seen_wpm_polarity {
                            return no_wpm_polarity_err;
                        }
                        frames.push(Frame::KeyingPerfectDit);
                    }
                    EncoderFrameType::KeyingPerfectDah => {
                        if !seen_wpm_polarity {
                            return no_wpm_polarity_err;
                        }
                        frames.push(Frame::KeyingPerfectDah);
                    }
                    EncoderFrameType::KeyingPerfectWordgap => {
                        if !seen_wpm_polarity {
                            return no_wpm_polarity_err;
                        }
                        frames.push(Frame::KeyingPerfectWordgap);
                    }
                    EncoderFrameType::KeyingEnd => {
                        if !seen_wpm_polarity {
                            return no_wpm_polarity_err;
                        }
                        frames.push(Frame::KeyingEnd);
                    }
                    EncoderFrameType::KeyingDeltaDit => {
                        if !seen_wpm_polarity {
                            return no_wpm_polarity_err;
                        }
                        let delta = extract_sized_delta(&mut extractor, timing.dit_encoding_range());
                        frames.push(Frame::KeyingDeltaDit { delta });
                    }
                    EncoderFrameType::KeyingDeltaDah => {
                        if !seen_wpm_polarity {
                            return no_wpm_polarity_err;
                        }
                        let delta = extract_sized_delta(&mut extractor, timing.dah_encoding_range());
                        frames.push(Frame::KeyingDeltaDah { delta });
                    }
                    EncoderFrameType::KeyingDeltaWordgap => {
                        if !seen_wpm_polarity {
                            return no_wpm_polarity_err;
                        }
                        let delta = extract_sized_delta(&mut extractor, timing.wordgap_encoding_range());
                        frames.push(Frame::KeyingDeltaWordgap { delta });
                    }
                    EncoderFrameType::KeyingNaive => {
                        if !seen_wpm_polarity {
                            return no_wpm_polarity_err;
                        }
                        let duration = extractor.extract_16_bits(11);
                        frames.push(Frame::KeyingNaive { duration });
                    }
                    EncoderFrameType::Unused => {
                        frames.push(Frame::Unused);
                    }
                    EncoderFrameType::Extension => {
                        frames.push(Frame::Extension);
                    }
                }
            }
        }
    }
    info!("Decoded {:?}", frames);
    Ok(frames)
}

fn extract_sized_delta(extractor: &mut BitvecSourceEncodingExtractor, encoding_range: (usize, usize)) -> i16 {
    let negative = extractor.extract_bool();
    let bits = if negative {
        encoding_range.0
    } else {
        encoding_range.1
    };
    let encoded = extractor.extract_16_bits(bits);
    let delta = decode_from_binary_with_known_sign(encoded, bits, !negative);
    delta
}


#[cfg(test)]
#[path = "./source_decoder_spec.rs"]
mod source_decoder_spec;
