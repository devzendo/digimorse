use log::{debug, info};
use std::sync::{Arc, RwLock};
use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyerSpeed, KeyingTimedEvent};
use crate::libs::source_encoder::source_encoding::{EncoderFrameType, SourceEncodingBuilder};

pub trait KeyingEncoder {
    /// Encode a KeyingTimedEvent in the Builder, in the most appropriate encoding (perfect, delta
    /// or naïve). If this encoding will fit, encode it and return true. If it won't fit, return
    /// false. The caller will then build() the Builder and emit it, and call again.
    fn encode_keying(&mut self, keying: KeyingTimedEvent) -> bool;
    // The KeyingEncoder needs to know the keyer speed to build keying frames into their most
    // compact form; a minimal delta from the three timing elements.
    fn set_keyer_speed(&mut self, speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;
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
    fn encode_keying(&mut self, keying: KeyingTimedEvent) -> bool {
        if self.keyer_speed == 0 {
            panic!("No speed has been set on the DefaultKeyingEncoder");
        }
        debug!("KeyingEncoder encoding {}", keying);
        // Can we use perfect encoding? Is this duration spot on?
        // TODO plus/minus some epsilon to quantise slightly, to pack more encoding in a block.
        let mut storage = self.storage.write().unwrap();
        if keying.duration == self.perfect_dit_ms {
            if storage.remaining() < 4 {
                return false
            } else {
                let frame_type = EncoderFrameType::KeyingPerfectDit;
                debug!("Adding {:?}", frame_type);
                storage.add_8_bits(frame_type as u8, 4);
            }
        } else if keying.duration == self.perfect_dah_ms {

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
}

#[cfg(test)]
#[path = "./keying_encoder_spec.rs"]
mod keying_encoder_spec;
