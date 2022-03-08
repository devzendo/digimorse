extern crate hamcrest2;

use log::debug;
use std::sync::{Arc, RwLock};

use crate::libs::keyer_io::keyer_io::KeyerSpeed;
use crate::libs::source_codec::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
use crate::libs::source_codec::keying_encoder::{DefaultKeyingEncoder, KeyingEncoder};
use crate::libs::source_codec::metadata_codec::{encode_callsign, encode_locator};
use crate::libs::source_codec::source_encoding::{EncoderFrameType, Frame, SourceEncodingBuilder};

/// Build a block of encoded data, not caring about overstuffing it since this
/// will be used in a test, and the builder panics if there's too much data.
/// This is just to offer a more comfortable test data creation facility than hand-constructing
/// arrays of binary bytes.
pub fn encoded(block_size_in_bits: usize, wpm: KeyerSpeed, frames: &[Frame]) -> Vec<u8> {
    let box_builder: Box<dyn SourceEncodingBuilder + Send + Sync> = Box::new(BitvecSourceEncodingBuilder::new(block_size_in_bits));
    let arc_locked_builder = Arc::new(RwLock::new(box_builder));
    let builder = arc_locked_builder.clone();
    let mut keying_encoder = DefaultKeyingEncoder::new(arc_locked_builder);
    keying_encoder.set_keyer_speed(wpm);
    for frame in frames {
        debug!("Encoding {:?}", frame);
        match frame {
            Frame::Padding => {
                // Frame type of Padding is 0000 so this'll look like padding
                let mut b = builder.write().unwrap();
                while b.remaining() > 0 {
                    b.add_bool(false);
                }
            }
            Frame::WPMPolarity { wpm, polarity } => {
                // Track speed changes by recalculating (at least) delta encoding sizes.
                keying_encoder.set_keyer_speed(*wpm);

                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::WPMPolarity as u8, 4);
                b.add_8_bits(*wpm as u8, 6);
                b.add_bool(*polarity);
            }
            Frame::CallsignMetadata { callsign } => {
                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::CallsignMetadata as u8, 4);
                b.add_32_bits(encode_callsign(callsign.clone()), 28);
            }
            Frame::CallsignHashMetadata { hash } => {
                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::CallsignHashMetadata as u8, 4);
                b.add_16_bits(*hash as u16, 16);
            }
            Frame::LocatorMetadata { locator } => {
                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::LocatorMetadata as u8, 4);
                b.add_16_bits(encode_locator(locator.clone()), 15);
            }
            Frame::KeyingPerfectDit => {
                keying_encoder.encode_perfect_dit();
            }
            Frame::KeyingPerfectDah => {
                keying_encoder.encode_perfect_dah();
            }
            Frame::KeyingPerfectWordgap => {
                keying_encoder.encode_perfect_wordgap();
            }
            Frame::KeyingEnd => {
                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::KeyingEnd as u8, 4);
            }
            Frame::KeyingDeltaDit { delta } => {
                keying_encoder.encode_delta_dit((*delta) as i16);
            }
            Frame::KeyingDeltaDah { delta } => {
                keying_encoder.encode_delta_dah((*delta) as i16);
            }
            Frame::KeyingDeltaWordgap { delta } => {
                keying_encoder.encode_delta_wordgap((*delta) as i16);
            }
            Frame::KeyingNaive { duration } => {
                keying_encoder.encode_naive(*duration);
            }
            Frame::Unused => {
                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::Unused as u8, 4);
            }
            Frame::Extension => {
                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::Extension as u8, 4);
            }
        }
    }
    let source_encoding = builder.write().unwrap().build();
    source_encoding.block
}


#[cfg(test)]
mod test_encoding_builder_spec {
    use log::debug;


    use crate::libs::source_codec::source_encoding::Frame;
    use crate::libs::source_codec::test_encoding_builder::encoded;
    use crate::libs::util::util::dump_byte_vec;
    use crate::libs::matchers::starts_with::*;

    use hamcrest2::prelude::*;

    const TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS: usize = 64;

    #[test]
    fn encode_wpm_polarity() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::WPMPolarity { wpm: 20, polarity: true },
        ]);
        assert_that!(&vec,
                   //
                   //                 F:WPWPM-    --P
                   starts_with(vec![0b00010101, 0b00100000, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_perfect_dit() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::KeyingPerfectDit,
        ]);
        assert_that!(&vec,
                   //                 F:PD
                   starts_with(vec![0b01100000, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_perfect_dah() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::KeyingPerfectDah,
        ]);
        assert_that!(&vec,
                   //                 F:PD
                   starts_with(vec![0b01110000, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_perfect_wordgap() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::KeyingPerfectWordgap,
        ]);
        assert_that!(&vec,
                   //                 F:PW
                   starts_with(vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_padding() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::Padding,
        ]);
        assert_that!(&vec,
                   //                 F:PA
                   starts_with(vec![0b00000000, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_end() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::KeyingEnd,
        ]);
        assert_that!(&vec,
                   //                 F:EN
                   starts_with(vec![0b10010000, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_delta_dit() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::KeyingDeltaDit { delta: 1 },
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_that!(&vec,
                   //                 F:DD
                   starts_with(vec![0b10100000, 0b00100000, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_delta_dah() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::KeyingDeltaDah { delta: 1 },
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_that!(&vec,
                   //                 F:DD
                   starts_with(vec![0b10110000, 0b00010000, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_delta_wordgap() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::KeyingDeltaWordgap { delta: 1 },
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_that!(&vec,
                   //                 F:DW
                   starts_with(vec![0b11000000, 0b00010000, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_naive() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
            Frame::KeyingNaive { duration: 16 },
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_that!(&vec,
                   //                 F:NE
                   starts_with(vec![0b11010000, 0b00100000, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn encode_tracks_speed_changes() {
        let vec = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 5, &[
            // These two speeds have different dah encoding sizes
            Frame::WPMPolarity { wpm: 5, polarity: true },
            Frame::KeyingDeltaDah { delta: 1 }, // 9 bits
            Frame::WPMPolarity { wpm: 60, polarity: true },
            Frame::KeyingDeltaDah { delta: -1 }, // 5 bits
            Frame::Extension, // to see a 4 bit end marker
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_that!(&vec,
                   //                                F:DD                  F:WPWPM    ---P              F:    EX
                   //                 F:WPWPM-    --P    S    DELTA---    -               F:DD    SDELTA
                   starts_with(vec![0b00010001, 0b01110110, 0b00000000, 0b10001111, 0b10011011, 0b11111111, 0b11000000, 0]));
    }
}

