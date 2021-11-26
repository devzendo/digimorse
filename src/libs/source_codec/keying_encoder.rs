use log::debug;
use std::sync::{Arc, RwLock};
use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyerSpeed, KeyingTimedEvent};
use crate::libs::source_codec::source_encoding::{EncoderFrameType, SourceEncodingBuilder};

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
}

pub struct DefaultKeyingEncoder {
    keyer_speed: KeyerSpeed,
    storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>,
    perfect_dit_ms: KeyerEdgeDurationMs,
    perfect_dah_ms: KeyerEdgeDurationMs,
    perfect_wordgap_ms: KeyerEdgeDurationMs,
}

impl DefaultKeyingEncoder {
    pub fn new(storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>) -> Self {
        Self {
            keyer_speed: 0,
            storage,
            perfect_dit_ms: 0,
            perfect_dah_ms: 0,
            perfect_wordgap_ms: 0
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
        } else {
            let dit = 1200 / speed as u16; // funky...
            self.perfect_dit_ms = dit as KeyerEdgeDurationMs;
            self.perfect_dah_ms = self.perfect_dit_ms * 3;
            self.perfect_wordgap_ms = self.perfect_dit_ms * 7;
        }
        debug!("KeyingEncoder speed set to {} WPM; dit: {}ms dah: {}ms wordgap: {}ms", self
            .keyer_speed,
            self.perfect_dit_ms, self.perfect_dah_ms, self.perfect_wordgap_ms)
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }

    fn encode_perfect_dit(&mut self) -> bool {
        let mut storage = self.storage.write().unwrap();
        if storage.remaining() < 4 {
            return false
        } else {
            let frame_type = EncoderFrameType::KeyingPerfectDit;
            debug!("Adding {:?}", frame_type);
            storage.add_8_bits(frame_type as u8, 4);
            return true
        }
    }

    fn encode_perfect_dah(&mut self) -> bool {
        let mut storage = self.storage.write().unwrap();
        if storage.remaining() < 4 {
            return false
        } else {
            let frame_type = EncoderFrameType::KeyingPerfectDah;
            debug!("Adding {:?}", frame_type);
            storage.add_8_bits(frame_type as u8, 4);
            return true
        }
    }

}

// From the table of delta encoding bit ranges per keying speed
pub fn dit_encoding_range(wpm: KeyerSpeed) -> (u8, u8) {
    if wpm >= 5 && wpm <= 9 {
        return (8, 8);
    } else if wpm >= 10 && wpm <= 18 {
        return (7, 7);
    } else if wpm >= 19 && wpm <= 37 {
        return (6, 6);
    } else if wpm <= 60 {
        return (5, 5)
    }
    panic!("WPM of {} is out of range in dit_encoding_range", wpm);
}

pub fn dah_encoding_range(wpm: KeyerSpeed) -> (u8, u8) {
    if wpm >= 5 && wpm <= 9 {
        return (8, 9);
    } else if wpm >= 10 && wpm <= 18 {
        return (7, 8);
    } else if wpm >= 19 && wpm <= 37 {
        return (6, 7);
    } else if wpm <= 60 {
        return (5, 6)
    }
    panic!("WPM of {} is out of range in dah_encoding_range", wpm);
}

pub fn wordgap_encoding_range(wpm: KeyerSpeed) -> (u8, u8) {
    if wpm >= 5 && wpm <= 9 {
        return (9, 9);
    } else if wpm >= 10 && wpm <= 18 {
        return (8, 8);
    } else if wpm >= 19 && wpm <= 37 {
        return (7, 7);
    } else if wpm <= 60 {
        return (6, 6)
    }
    panic!("WPM of {} is out of range in wordgap_encoding_range", wpm);
}

#[cfg(test)]
#[path = "./keying_encoder_spec.rs"]
mod keying_encoder_spec;
