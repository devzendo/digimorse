use std::sync::mpsc::Receiver;

use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed};

/*
 * Ideas...
 * Batch up encodings to some maximum size. Can batches be emitted as a byte stream?
 * Track up/down key state, send the current state as the first encoding in a batch so that the
 * receiver knows whether it's mark or space, per-batch. If a batch does not decode correctly
 * the receiver will miss that batch and need to re-sync its idea of mark/space.
 *
 * If we're aiming for a wide range of WPM speeds, 5 to 40WPM, these have a wide range of dit/dah
 * element durations in ms.
 * A dit at 40WPM is ___ms (and it could be sent short).
 * A dah at 5WPM is __ms (and it could be sent long).
 * So a range of __ms to __ms.
 * WHAT DOES THE LDPC (CHANNEL ENCODER) REQUIRE AS ITS INPUT?
 */
#[derive(Clone, PartialEq)]
pub enum SourceEncoding {
    // Timed(KeyingTimedEvent),
    // Start(),
    // End(),
    // just don't know yet what this will include.
}

pub trait SourceEncoder {
    fn set_keyer_speed(&mut self, speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;
}

#[readonly::make]
pub struct DefaultSourceEncoder {
    keyer_speed: KeyerSpeed,
    keying_event_rx: Receiver<KeyingEvent>,

}

impl DefaultSourceEncoder {
    pub fn new(keying_event_rx: Receiver<KeyingEvent>) -> Self {
        Self {
            keyer_speed: 12,
            keying_event_rx
        }
    }
}

impl SourceEncoder for DefaultSourceEncoder {
    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keyer_speed = speed;
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }
}

#[cfg(test)]
#[path = "./source_encoder_spec.rs"]
mod source_encoder_spec;
