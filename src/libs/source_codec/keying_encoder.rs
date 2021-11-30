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

    /// Obtain the perfect timings, for the current keyer speed.
    fn get_perfect_dit_ms(&self) -> KeyerEdgeDurationMs;
    fn get_perfect_dah_ms(&self) -> KeyerEdgeDurationMs;
    fn get_perfect_wordgap_ms(&self) -> KeyerEdgeDurationMs;

    /// Obtain the delta ranges, for the current keyer speed.
    fn get_dit_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);
    fn get_dah_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);
    fn get_wordgap_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);

    // Routines used internally by the KeyingEncoder, and also reused by tests. All return true if
    // the encoding will fit, false if it won't.
    fn encode_perfect_dit(&mut self) -> bool;
    fn encode_perfect_dah(&mut self) -> bool;
    fn encode_perfect_wordgap(&mut self) -> bool;
    fn encode_delta_dit(&mut self, delta: i16) -> bool;
    fn encode_delta_dah(&mut self, delta: i16) -> bool;
    fn encode_delta_wordgap(&mut self, delta: i16) -> bool;
    fn encode_naive(&mut self, duration: KeyerEdgeDurationMs) -> bool;
}


pub struct DefaultKeyingEncoder {
    keyer_speed: KeyerSpeed,
    storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>,
    perfect_dit_ms: KeyerEdgeDurationMs,
    perfect_dah_ms: KeyerEdgeDurationMs,
    perfect_wordgap_ms: KeyerEdgeDurationMs,
    negative_dit_range: KeyerRangeDelta,
    positive_dit_range: KeyerRangeDelta,
    lower_dit_bound: KeyerEdgeDurationMs,
    upper_dit_bound: KeyerEdgeDurationMs,
    negative_dah_range: KeyerRangeDelta,
    positive_dah_range: KeyerRangeDelta,
    lower_dah_bound: KeyerEdgeDurationMs,
    upper_dah_bound: KeyerEdgeDurationMs,
    negative_wordgap_range: KeyerRangeDelta,
    positive_wordgap_range: KeyerRangeDelta,
    lower_wordgap_bound: KeyerEdgeDurationMs,
    upper_wordgap_bound: KeyerEdgeDurationMs,
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
            lower_dit_bound: 0,
            upper_dit_bound: 0,
            negative_dah_range: 0,
            positive_dah_range: 0,
            lower_dah_bound: 0,
            upper_dah_bound: 0,
            negative_wordgap_range: 0,
            positive_wordgap_range: 0,
            lower_wordgap_bound: 0,
            upper_wordgap_bound: 0
        }
    }

    fn encode_perfect_frame(&mut self, frame_type: EncoderFrameType) -> bool {
        let mut storage = self.storage.write().unwrap();
        let remaining = storage.remaining();
        return if remaining < 4 {
            debug!("Insufficient storage ({}) to add {:?}", remaining, frame_type);
            false
        } else {
            debug!("Adding {:?} (remaining before:{})", frame_type, remaining);
            storage.add_8_bits(frame_type as u8, 4);
            true
        }
    }

    fn encode_delta_frame(&mut self, frame_type: EncoderFrameType, delta: i16, encoding_range: (usize, usize)) -> bool {
        let mut storage = self.storage.write().unwrap();
        let remaining = storage.remaining();
        let bits;
        if delta < 0 {
            bits = encoding_range.0
        } else {
            bits = encoding_range.1
        }
        // The full frame size contains the frame type (4), the bits required for the delta (bits)
        // plus a sign bit.
        let full_frame_size = 4 + bits + 1;
        debug!("Full frame of delta encoding is {} bits; {} remain", full_frame_size, remaining);
        return if remaining < full_frame_size {
            debug!("Insufficient storage ({}) to add {} bits of {:?}", remaining, full_frame_size, frame_type);
            false
        } else {
            debug!("Adding {:?} (remaining before:{})", frame_type, remaining);
            storage.add_8_bits(frame_type as u8, 4);
            // delta can't be 0, else this would be encoded as a perfect
            storage.add_16_bits(encode_to_binary(delta, bits), bits + 1); // +1 is the sign
            true
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
            if keying.duration >= self.lower_dit_bound && keying.duration <= self.upper_dit_bound {
                // It's a delta dit
                return self.encode_delta_dit(keying.duration as i16 - self.perfect_dit_ms as i16);
            } else if keying.duration >= self.lower_dah_bound && keying.duration <= self.upper_dah_bound {
                // It's a delta dah
                return self.encode_delta_dah(keying.duration as i16 - self.perfect_dah_ms as i16);
            } else if keying.duration >= self.lower_wordgap_bound && keying.duration <= self.upper_wordgap_bound {
                // It's a delta wordgap
                return self.encode_delta_wordgap(keying.duration as i16 - self.perfect_wordgap_ms as i16);
            } else {
                // Nope, use naïve encoding.
                return self.encode_naive(keying.duration);
            }
        }
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

            self.lower_dit_bound = 0;
            self.upper_dit_bound = 0;
            self.lower_dah_bound = 0;
            self.upper_dah_bound = 0;
            self.lower_wordgap_bound = 0;
            self.upper_wordgap_bound = 0;
        } else {
            let decimal_dit_ms = 1200f32 / speed as f32;
            let decimal_dah_ms = decimal_dit_ms * 3f32;
            let decimal_wordgap_ms = decimal_dit_ms * 7f32;
            //debug!("decimal_dit_ms is {}", decimal_dit_ms);
            let dit = decimal_dit_ms as u16; // funky...
            self.perfect_dit_ms = decimal_dit_ms as KeyerEdgeDurationMs;
            self.perfect_dah_ms = decimal_dah_ms as KeyerEdgeDurationMs;
            self.perfect_wordgap_ms = decimal_wordgap_ms as KeyerEdgeDurationMs;
            // Delta ranges are based off midpoints between the perfect dit/dah/wordgap. The maximum
            // is capped at 367, not 480 since wordgap+367=2047 which fits in 11 bits. Slow delta
            // wordgaps above 367 would be encoded as a naïve.
            // See docs/Morse speeds.xlsx for the derivations of these.
            let dit_dah_midpoint = decimal_dah_ms - decimal_dit_ms;
            let dah_wordgap_midpoint = decimal_dah_ms + ((decimal_wordgap_ms - decimal_dah_ms)/2f32);
            //debug!("dit_dah_midpoint {}, dah_wordgap_midpoint {}", dit_dah_midpoint, dah_wordgap_midpoint);
            // There will be non-null intersections between the three, so I've shrunk the middle dah
            // range on either end by one. Then the three ranges are disjoint. This diverges from
            // the docs/Morse speeds.xlsx spreadsheet.
            self.negative_dit_range = -(dit as i16);
            self.positive_dit_range = dit as i16;
            self.negative_dah_range = -(decimal_dah_ms - dit_dah_midpoint) as i16 + 1;
            self.positive_dah_range = (dah_wordgap_midpoint - decimal_dah_ms) as i16 - 1;
            self.negative_wordgap_range = -(decimal_wordgap_ms - dah_wordgap_midpoint) as i16;
            self.positive_wordgap_range = min(367, -(self.negative_wordgap_range));
            // Delta encoding bounds...
            let dit_i16 = self.perfect_dit_ms as i16;
            let dah_i16 = self.perfect_dah_ms as i16;
            let wordgap_i16 = self.perfect_wordgap_ms as i16;
            self.lower_dit_bound = (dit_i16 + self.negative_dit_range) as KeyerEdgeDurationMs;
            self.upper_dit_bound = (dit_i16 + self.positive_dit_range) as KeyerEdgeDurationMs;
            self.lower_dah_bound = (dah_i16 + self.negative_dah_range) as KeyerEdgeDurationMs;
            self.upper_dah_bound = (dah_i16 + self.positive_dah_range) as KeyerEdgeDurationMs;
            self.lower_wordgap_bound = (wordgap_i16 + self.negative_wordgap_range) as KeyerEdgeDurationMs;
            self.upper_wordgap_bound = (wordgap_i16 + self.positive_wordgap_range) as KeyerEdgeDurationMs;
        }
        debug!("KeyingEncoder speed set to {} WPM; dit: {}ms dah: {}ms wordgap: {}ms", self
            .keyer_speed,
            self.perfect_dit_ms, self.perfect_dah_ms, self.perfect_wordgap_ms);
        debug!("Delta dit [{} .. {}] dah [{} .. {}], wordgap [{} .. {}]",
            self.negative_dit_range, self.positive_dit_range,
            self.negative_dah_range, self.positive_dah_range,
            self.negative_wordgap_range, self.positive_wordgap_range
        );
        debug!("Duration dit [{} .. {}]ms dah [{} .. {}]ms wordgap [{} .. {}]ms",
            self.lower_dit_bound, self.upper_dit_bound,
            self.lower_dah_bound, self.upper_dah_bound,
            self.lower_wordgap_bound, self.upper_wordgap_bound);
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }

    fn get_perfect_dit_ms(&self) -> KeyerEdgeDurationMs {
        self.perfect_dit_ms
    }

    fn get_perfect_dah_ms(&self) -> KeyerEdgeDurationMs {
        self.perfect_dah_ms
    }

    fn get_perfect_wordgap_ms(&self) -> KeyerEdgeDurationMs {
        self.perfect_wordgap_ms
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

    fn encode_perfect_dit(&mut self) -> bool {
        self.encode_perfect_frame(EncoderFrameType::KeyingPerfectDit)
    }

    fn encode_perfect_dah(&mut self) -> bool {
        self.encode_perfect_frame(EncoderFrameType::KeyingPerfectDah)
    }

    fn encode_perfect_wordgap(&mut self) -> bool {
        self.encode_perfect_frame(EncoderFrameType::KeyingPerfectWordgap)
    }

    fn encode_delta_dit(&mut self, delta: i16) -> bool {
        self.encode_delta_frame(EncoderFrameType::KeyingDeltaDit, delta, dit_encoding_range(self.keyer_speed))
    }

    fn encode_delta_dah(&mut self, delta: i16) -> bool {
        self.encode_delta_frame(EncoderFrameType::KeyingDeltaDah, delta, dah_encoding_range(self.keyer_speed))
    }

    fn encode_delta_wordgap(&mut self, delta: i16) -> bool {
        self.encode_delta_frame(EncoderFrameType::KeyingDeltaWordgap, delta, wordgap_encoding_range(self.keyer_speed))
    }

    fn encode_naive(&mut self, duration: KeyerEdgeDurationMs) -> bool {
        if duration > 2047 {
            panic!("Duration of {} cannot be encoded in 11 bits", duration);
        }
        let mut storage = self.storage.write().unwrap();
        let remaining = storage.remaining();
        return if remaining < 15 {
            debug!("Insufficient storage ({}) to add 15 bits of {:?}", remaining, EncoderFrameType::KeyingNaive);
            false
        } else {
            debug!("Adding {:?} duration {} (remaining before:{})", EncoderFrameType::KeyingNaive, duration, remaining);
            storage.add_8_bits(EncoderFrameType::KeyingNaive as u8, 4);
            // delta can't be 0, else this would be encoded as a perfect
            storage.add_16_bits(duration, 11);
            true
        }
    }
}

// From the table of delta encoding bit ranges per keying speed
pub fn dit_encoding_range(wpm: KeyerSpeed) -> (usize, usize) {
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

pub fn dah_encoding_range(wpm: KeyerSpeed) -> (usize, usize) {
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

pub fn wordgap_encoding_range(wpm: KeyerSpeed) -> (usize, usize) {
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

/// Given a number in delta range [-480 .. 480] (see note below), encode it in a given number of
/// bits, returning it in a suitably sized type. This type can have enough of its rightmost bits
/// taken to represent the encoded delta.
/// The output type must be large enough to encode a sign bit plus enough bits to encode 480
/// (1+9 bits); ie a u16. Negative deltas are encoded using 2's complement binary.
/// The bits parameter does not include the sign bit, but the output does. The output is not
/// sign extended - i.e. for negative outputs, only the sign bit in the output is 1: bits to the
/// left of the sign bit are 0. Users of this should take the 1+bits rightmost bits of the output
/// to obtain the encoding including sign.
/// The input range [-480 .. 480] is formed by taking the union of all the delta ranges, i.e. it
/// has upper and lower bounds large enough to hold any delta encoding.
pub fn encode_to_binary(delta: i16, bits: usize) -> u16 {
    if delta < -480 || delta > 480 {
        panic!("Cannot encode an out of range delta ({})", delta);
    }
    if bits < 5 || bits > 9 {
        panic!("Cannot encode with an out of range number of bits ({})", bits);
    }
    let out_of_range =
        (bits == 9 && (delta < -480 || delta > 480)) ||
        (bits == 8 && (delta < -240 || delta > 240)) ||
        (bits == 7 && (delta < -127 || delta > 127)) ||
        (bits == 6 && (delta < -63  || delta > 63)) ||
        (bits == 5 && (delta < -31  || delta > 31));
    if out_of_range {
        panic!("Cannot encode delta {} in {} bits", delta, bits);
    }
    debug!("Encoding delta {} in {} bits", delta, bits);

    if delta >= 0 {
        delta as u16
    } else {
        let mask = mask_n_bits(bits + 1);
        debug!("<=0    mask is {:#018b}", mask);

        let ret = (delta as u16) & mask;
        debug!("<=0 encoded as {:#018b}", ret);
        ret
    }
}

fn mask_n_bits(bits: usize) -> u16 {
    let mut mask = 0u16;
    for _ in 0..bits {
        mask = (mask << 1) | 1;
    }
    mask
}

/// Inverse of encode_to_binary. Given an encoded value and the number of bits of its value (ie not
/// including the sign bit), return the signed value encoded in the bits+1 rightmost bits of
/// encoded. The output will be truncated to the range [-480 .. 480].
pub fn decode_from_binary(encoded: u16, bits: usize) -> i16 {
    if bits < 5 || bits > 9 {
        panic!("Cannot decode with an out of range number of bits ({})", bits);
    }
    let mask = mask_n_bits(bits);
    let mut sign = 1u16;
    for _ in 0..bits {
        sign <<= 1;
    }
    debug!("encoded        {:#018b}", encoded);
    debug!("sign           {:#018b}", sign);
    debug!("mask           {:#018b}", mask);
    let mut ret;
    if encoded & sign == 0 {
        ret = encoded as i16;
    } else {
        if encoded == 0 {
            ret = 0
        } else {
            ret = -((!(encoded - 1) & mask) as i16);
        }
    }
    if ret > 480 {
        debug!("decoded ({}) > 480; truncating", ret);
        ret = 480;
    } else if ret < -480 {
        debug!("decoded ({}) < -480; truncating", ret);
        ret = -480;
    }
    debug!("decoded        {:#016b} ({})", ret, ret);
    ret
}

#[cfg(test)]
#[path = "./keying_encoder_spec.rs"]
mod keying_encoder_spec;
