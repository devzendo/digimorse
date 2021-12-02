extern crate hamcrest2;

use std::sync::{Arc, RwLock};
use crate::libs::keyer_io::keyer_io::KeyerSpeed;
use crate::libs::source_codec::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
use crate::libs::source_codec::keying_encoder::{DefaultKeyingEncoder, KeyingEncoder};
use crate::libs::source_codec::metadata_codec::{encode_callsign, encode_locator};
use crate::libs::source_codec::source_encoding::{Callsign, CallsignHash, EncoderFrameType, KeyingDelta, KeyingNaive, Locator, SourceEncodingBuilder};

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

/// Build a block of encoded data, not caring about overstuffing it since this
/// will be used in a test, and the builder panics if there's too much data.
/// This is just to offer a more comfortable test data creation facility than hand-constructing
/// arrays of binary bytes.
pub fn encoded(wpm: KeyerSpeed, frames: &[Frame]) -> Vec<u8> {
    let box_builder: Box<dyn SourceEncodingBuilder + Send + Sync> = Box::new(BitvecSourceEncodingBuilder::new());
    let arc_locked_builder = Arc::new(RwLock::new(box_builder));
    let builder = arc_locked_builder.clone();
    let mut keying_encoder = DefaultKeyingEncoder::new(arc_locked_builder);
    keying_encoder.set_keyer_speed(wpm);
    for frame in frames {
        match frame {
            Frame::Padding => {
                // Frame type of Padding is 0000 so this'll look like padding
                let mut b = builder.write().unwrap();
                while b.remaining() > 0 {
                    b.add_bool(false);
                }
            }
            Frame::WPMPolarity { wpm, polarity } => {
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
                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::KeyingPerfectDah as u8, 4);
            }
            Frame::KeyingPerfectWordgap => {
                let mut b = builder.write().unwrap();
                b.add_8_bits(EncoderFrameType::KeyingPerfectWordgap as u8, 4);
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
    use crate::libs::source_codec::test_encoding_builder::{encoded, Frame};
    use crate::libs::util::util::dump_byte_vec;

    #[test]
    fn encode_wpm_polarity() {
        let vec = encoded(20, &[
            Frame::WPMPolarity { wpm: 20, polarity: true },
        ]);
        assert_eq!(vec,
                   //
                   //     F:WPWPM-    --P
                   vec![0b00010101, 0b00100000, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_perfect_dit() {
        let vec = encoded(20, &[
            Frame::KeyingPerfectDit,
        ]);
        assert_eq!(vec,
                   //     F:PD
                   vec![0b01100000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_perfect_dah() {
        let vec = encoded(20, &[
            Frame::KeyingPerfectDah,
        ]);
        assert_eq!(vec,
                   //     F:PD
                   vec![0b01110000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_perfect_wordgap() {
        let vec = encoded(20, &[
            Frame::KeyingPerfectWordgap,
        ]);
        assert_eq!(vec,
                   //     F:PW
                   vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_padding() {
        let vec = encoded(20, &[
            Frame::Padding,
        ]);
        assert_eq!(vec,
                   //     F:PA
                   vec![0b00000000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_end() {
        let vec = encoded(20, &[
            Frame::KeyingEnd,
        ]);
        assert_eq!(vec,
                   //     F:EN
                   vec![0b10010000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_delta_dit() {
        let vec = encoded(20, &[
            Frame::KeyingDeltaDit { delta: 1 },
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_eq!(vec,
                   //     F:DD
                   vec![0b10100000, 0b00100000, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_delta_dah() {
        let vec = encoded(20, &[
            Frame::KeyingDeltaDah { delta: 1 },
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_eq!(vec,
                   //     F:DD
                   vec![0b10110000, 0b00010000, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_delta_wordgap() {
        let vec = encoded(20, &[
            Frame::KeyingDeltaWordgap { delta: 1 },
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_eq!(vec,
                   //     F:DW
                   vec![0b11000000, 0b00010000, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn encode_naive() {
        let vec = encoded(20, &[
            Frame::KeyingNaive { duration: 16 },
        ]);
        debug!("{}", dump_byte_vec(&vec));
        assert_eq!(vec,
                   //     F:NE
                   vec![0b11010000, 0b00100000, 0, 0, 0, 0, 0, 0]);
    }
}

