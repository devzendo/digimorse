use log::debug;
use std::sync::{Arc, RwLock};
use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyerSpeed, KeyingTimedEvent};
use crate::libs::source_codec::keying_timing::{DefaultKeyingTiming, KeyingTiming};
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
    storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>,
    timing: DefaultKeyingTiming,
    keyer_speed: KeyerSpeed,
}

impl DefaultKeyingEncoder {
    pub fn new(storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>) -> Self {
        Self {
            keyer_speed: 0,
            timing: DefaultKeyingTiming::new(),
            storage,
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
    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keyer_speed = speed;
        self.timing.set_keyer_speed(speed);
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }

    fn encode_keying(&mut self, keying: &KeyingTimedEvent) -> bool {
        if self.keyer_speed == 0 {
            panic!("No speed has been set on the DefaultKeyingEncoder");
        }
        debug!("KeyingEncoder encoding {}", keying);
        // Can we use perfect encoding? Is this duration spot on?
        // TODO plus/minus some epsilon to quantise slightly, to pack more encoding in a block.
        if keying.duration == self.timing.get_perfect_dit_ms() {
            return self.encode_perfect_dit();
        } else if keying.duration == self.timing.get_perfect_dah_ms() {
            return self.encode_perfect_dah();
        } else if keying.duration == self.timing.get_perfect_wordgap_ms() {
            return self.encode_perfect_wordgap();
        } else {
            // Can we use delta encoding? Is this duration within the ranges?
            if keying.duration >= self.timing.get_lower_dit_bound() && keying.duration <= self.timing.get_upper_dit_bound() {
                // It's a delta dit
                return self.encode_delta_dit(keying.duration as i16 - self.timing.get_perfect_dit_ms() as i16);
            } else if keying.duration >= self.timing.get_lower_dah_bound() && keying.duration <= self.timing.get_upper_dah_bound() {
                // It's a delta dah
                return self.encode_delta_dah(keying.duration as i16 - self.timing.get_perfect_dah_ms() as i16);
            } else if keying.duration >= self.timing.get_lower_wordgap_bound() && keying.duration <= self.timing.get_upper_wordgap_bound() {
                // It's a delta wordgap
                return self.encode_delta_wordgap(keying.duration as i16 - self.timing.get_perfect_wordgap_ms() as i16);
            } else {
                // Nope, use naïve encoding.
                return self.encode_naive(keying.duration);
            }
        }
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
        self.encode_delta_frame(EncoderFrameType::KeyingDeltaDit, delta, self.timing.dit_encoding_range())
    }

    fn encode_delta_dah(&mut self, delta: i16) -> bool {
        self.encode_delta_frame(EncoderFrameType::KeyingDeltaDah, delta, self.timing.dah_encoding_range())
    }

    fn encode_delta_wordgap(&mut self, delta: i16) -> bool {
        self.encode_delta_frame(EncoderFrameType::KeyingDeltaWordgap, delta, self.timing.wordgap_encoding_range())
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
