use std::cmp::min;
use log::debug;
use std::sync::{Arc, RwLock};
use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyerSpeed, KeyingTimedEvent};
use crate::libs::source_codec::source_encoding::{EncoderFrameType, SourceEncodingBuilder};

pub type KeyerRangeDelta = i16;

pub trait KeyingEncoder {
    /// Encode a KeyingTimedEvent in the Builder, in the most appropriate encoding (perfect, delta
    /// or naïve). If this encoding will fit, encode it and return true. If it won't fit, return
    /// false. The caller will then build() the Builder and emit it, and call again.
    fn encode_keying(&mut self, keying: &KeyingTimedEvent) -> bool;
    // The KeyingEncoder needs to know the keyer speed to build keying frames into their most
    // compact form; a minimal delta from the three timing elements.
    fn set_keyer_speed(&mut self, speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;

    /// Obtain the delta ranges, for the current keyer speed.
    fn get_dit_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);
    fn get_dah_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);
    fn get_wordgap_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);

    // Routines used internally by the KeyingEncoder, and also reused by tests. All return true if
    // the encoding will fit, false if it won't.
    fn encode_perfect_dit(&mut self) -> bool;
    fn encode_perfect_dah(&mut self) -> bool;
    fn encode_perfect_wordgap(&mut self) -> bool;
}


pub struct DefaultKeyingEncoder {
    keyer_speed: KeyerSpeed,
    storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>,
    perfect_dit_ms: KeyerEdgeDurationMs,
    perfect_dah_ms: KeyerEdgeDurationMs,
    perfect_wordgap_ms: KeyerEdgeDurationMs,
    negative_dit_range: KeyerRangeDelta,
    positive_dit_range: KeyerRangeDelta,
    negative_dah_range: KeyerRangeDelta,
    positive_dah_range: KeyerRangeDelta,
    negative_wordgap_range: KeyerRangeDelta,
    positive_wordgap_range: KeyerRangeDelta,
}

impl DefaultKeyingEncoder {
    pub fn new(storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>) -> Self {
        Self {
            keyer_speed: 0,
            storage,
            perfect_dit_ms: 0,
            perfect_dah_ms: 0,
            perfect_wordgap_ms: 0,
            negative_dit_range: 0,
            positive_dit_range: 0,
            negative_dah_range: 0,
            positive_dah_range: 0,
            negative_wordgap_range: 0,
            positive_wordgap_range: 0,
        }
    }

    fn encode_perfect_frame(&mut self, frame_type: EncoderFrameType) -> bool {
        let mut storage = self.storage.write().unwrap();
        let remaining = storage.remaining();
        if remaining < 4 {
            debug!("Insufficient storage ({}) to add {:?}", remaining, frame_type);
            return false
        } else {
            debug!("Adding {:?} (remaining before:{})", frame_type, remaining);
            storage.add_8_bits(frame_type as u8, 4);
            return true
        }
    }
}

impl KeyingEncoder for DefaultKeyingEncoder {
    fn encode_keying(&mut self, keying: &KeyingTimedEvent) -> bool {
        if self.keyer_speed == 0 {
            panic!("No speed has been set on the DefaultKeyingEncoder");
        }
        debug!("KeyingEncoder encoding {}", keying);
        // Can we use perfect encoding? Is this duration spot on?
        // TODO plus/minus some epsilon to quantise slightly, to pack more encoding in a block.
        if keying.duration == self.perfect_dit_ms {
            return self.encode_perfect_dit();
        } else if keying.duration == self.perfect_dah_ms {
            return self.encode_perfect_dah();
        } else if keying.duration == self.perfect_wordgap_ms {
            return self.encode_perfect_wordgap();
        } else {
            // Can we use delta encoding? Is this duration within the ranges?
            // Nope, use naïve encoding.
        }
        true
    }

    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keyer_speed = speed;
        // Allow setting it to zero to aid tests
        if self.keyer_speed == 0 {
            self.perfect_dit_ms = 0;
            self.perfect_dah_ms = 0;
            self.perfect_wordgap_ms = 0;
            self.negative_dit_range = 0;
            self.positive_dit_range = 0;
            self.negative_dah_range = 0;
            self.positive_dah_range = 0;
            self.negative_wordgap_range = 0;
            self.positive_wordgap_range = 0;

        } else {
            let dit = 1200 / speed as u16; // funky...
            self.perfect_dit_ms = dit as KeyerEdgeDurationMs;
            self.perfect_dah_ms = self.perfect_dit_ms * 3;
            self.perfect_wordgap_ms = self.perfect_dit_ms * 7;
            // Delta ranges are based off midpoints between the perfect dit/dah/wordgap. The maximum
            // is capped at 367, not 480 since wordgap+367=2047 which fits in 11 bits. Slow delta
            // wordgaps above 367 would be encoded as a naïve.
            // See docs/Morse speeds.xlsx for the derivations of these.
            let dit_dah_midpoint = (self.perfect_dah_ms - self.perfect_dit_ms) as i16;
            let dah_wordgap_midpoint = (self.perfect_dah_ms + ((self.perfect_wordgap_ms - self.perfect_dah_ms)/2)) as i16;
            self.negative_dit_range = -(dit as i16);
            self.positive_dit_range = dit as i16;
            self.negative_dah_range = -(self.perfect_dah_ms as i16 - dit_dah_midpoint);
            self.positive_dah_range = dah_wordgap_midpoint - self.perfect_dah_ms as i16;
            self.negative_wordgap_range = -(self.perfect_wordgap_ms as i16 - dah_wordgap_midpoint);
            self.positive_wordgap_range = min(367, -(self.negative_wordgap_range));
        }
        debug!("KeyingEncoder speed set to {} WPM; dit: {}ms dah: {}ms wordgap: {}ms", self
            .keyer_speed,
            self.perfect_dit_ms, self.perfect_dah_ms, self.perfect_wordgap_ms);
        debug!("Delta dit ({}, {}) dah ({}, {}), wordgap ({}, {})",
            self.negative_dit_range, self.positive_dit_range,
            self.negative_dah_range, self.positive_dah_range,
            self.negative_wordgap_range, self.positive_wordgap_range
        );
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }

    fn encode_perfect_dit(&mut self) -> bool {
        self.encode_perfect_frame(EncoderFrameType::KeyingPerfectDit)
    }

    fn encode_perfect_dah(&mut self) -> bool {
        self.encode_perfect_frame(EncoderFrameType::KeyingPerfectDah)
    }

    fn encode_perfect_wordgap(&mut self) -> bool {
        self.encode_perfect_frame(EncoderFrameType::KeyingPerfectWordgap)
    }

    fn get_dit_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta) {
        (self.negative_dit_range, self.positive_dit_range)
    }

    fn get_dah_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta) {
        (self.negative_dah_range, self.positive_dah_range)
    }

    fn get_wordgap_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta) {
        (self.negative_wordgap_range, self.positive_wordgap_range)
    }
}

// From the table of delta encoding bit ranges per keying speed
pub fn dit_encoding_range(wpm: KeyerSpeed) -> (u8, u8) {
    if wpm >= 5 {
        if wpm <= 9 {
            return (8, 8);
        } else if wpm >= 10 && wpm <= 18 {
            return (7, 7);
        } else if wpm >= 19 && wpm <= 37 {
            return (6, 6);
        } else if wpm <= 60 {
            return (5, 5)
        }
    }
    panic!("WPM of {} is out of range in dit_encoding_range", wpm);
}

pub fn dah_encoding_range(wpm: KeyerSpeed) -> (u8, u8) {
    if wpm >= 5 {
        if wpm <= 9 {
            return (8, 9);
        } else if wpm >= 10 && wpm <= 18 {
            return (7, 8);
        } else if wpm >= 19 && wpm <= 37 {
            return (6, 7);
        } else if wpm <= 60 {
            return (5, 6)
        }
    }
    panic!("WPM of {} is out of range in dah_encoding_range", wpm);
}

pub fn wordgap_encoding_range(wpm: KeyerSpeed) -> (u8, u8) {
    if wpm >= 5 {
        if wpm <= 9 {
            return (9, 9);
        } else if wpm >= 10 && wpm <= 18 {
            return (8, 8);
        } else if wpm >= 19 && wpm <= 37 {
            return (7, 7);
        } else if wpm <= 60 {
            return (6, 6)
        }
    }
    panic!("WPM of {} is out of range in wordgap_encoding_range", wpm);
}

/// Given a number in the union of the largest delta range [-480 .. 480], encode it in a number of
/// bits, returning it in a larger type that can that have quantity of its rightmost bits taken.
/// The output type must be large enough to encode a sign bit plus enough bits to encode 480
/// (1+9 bits); ie a u16. Negative deltas are encoded using 2's complement binary.
/// The bits parameter does not include the sign bit, but the output does. The output is not
/// necessarily sign extended. Users of this should take the 1+bits rightmost bits of the output
/// to obtain the encoding including sign.
pub fn encode_to_binary(delta: i16, bits: u8) -> u16 {
    0
}

/// Inverse of encode_to_binary. Given an encoded value and the number of bits of its value (ie not
/// including the sign bit), return the signed value encoded in the bits+1 rightmost bits of
/// encoded. The output will be truncated to the range [-480 .. 480].
pub fn decode_from_binary(encoded: u16, bits: u8) -> i16 {
    0
}

#[cfg(test)]
#[path = "./keying_encoder_spec.rs"]
mod keying_encoder_spec;
