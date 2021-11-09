use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use bus::{Bus, BusReader};
use bytes::BufMut;
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

// Size of all source encoder frames; could change as the design of later stages evolves.
const SOURCE_ENCODER_FRAME_SIZE: u16 = 64;

#[derive(Clone, PartialEq)]
pub struct SourceEncoding {
    // bytes of a frame
    pub frame: Vec<u8>,
    // Is this encoding frame the last in the sequence?
    pub isEnd: bool,
    // just don't know yet what this will include.
}

pub trait SourceEncoder {
    // The SourceEncoder needs to know the keyer speed to build keying frames into their most
    // compact form; a minimal delta from the three timing elements.
    fn set_keyer_speed(&mut self, speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;

    // Irrespective of how full the current frame is, pad it to SOURCE_ENCODER_FRAME_SIZE and emit
    // it.
    fn emit(&mut self);
}

#[readonly::make]
pub struct DefaultSourceEncoder {
    keyer_speed: KeyerSpeed,
    keying_event_rx: BusReader<KeyingEvent>,
    source_encoder_tx: Bus<SourceEncoding>,
    terminate: Arc<AtomicBool>
}

impl DefaultSourceEncoder {
    pub fn new(keying_event_rx: BusReader<KeyingEvent>, source_encoder_tx: Bus<SourceEncoding>, terminate: Arc<AtomicBool>) -> Self {
        Self {
            keyer_speed: 12,
            keying_event_rx,
            source_encoder_tx,
            terminate
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

    fn emit(&mut self) {
        todo!()
    }
}

#[cfg(test)]
#[path = "./source_encoder_spec.rs"]
mod source_encoder_spec;
