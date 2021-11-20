use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use bus::{Bus, BusReader};
use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed};
use crate::libs::source_encoder::source_encoding::SourceEncoding;

/*
 * Ideas...
 * Batch up encodings to some maximum block size. Can batches be emitted as a byte stream?
 * Track up/down key state, send the current state as the first encoding in a block so that the
 * receiver knows whether it's mark or space, per-block. If a block does not decode correctly
 * the receiver will miss that block and need to re-sync its idea of mark/space.
 *
 * If we're aiming for a wide range of WPM speeds, 5 to 60WPM, these have a wide range of dit/dah
 * element durations in ms.
 * A dit at 60WPM is 20ms (and it could be sent short).
 * A wordgap at 5WPM is 1680ms (and it could be sent long).
 * So a range of 20ms to 1680ms.
 */

pub trait SourceEncoder {
    // The SourceEncoder needs to know the keyer speed to build keying frames into their most
    // compact form; a minimal delta from the three timing elements.
    fn set_keyer_speed(&mut self, speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;

    // Irrespective of how full the current frame is, pad it to SOURCE_ENCODER_BLOCK_SIZE and emit
    // it on the output Bus<SourceEncoding>.
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
