use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::thread;
use std::thread::JoinHandle;
use bus::{Bus, BusReader};
use log::{debug};
use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed};
use crate::libs::source_encoder::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
use crate::libs::source_encoder::source_encoding::{SourceEncoding, SourceEncodingBuilder};

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
    // keying_event_rx: BusReader<KeyingEvent>,
    source_encoder_tx: Bus<SourceEncoding>,
    terminate: Arc<AtomicBool>,
    storage: Arc<Box<dyn SourceEncodingBuilder + Send + Sync>>, // ?? Is it Send + Sync?
    // Send + Sync are here so the DefaultSourceEncoder can be stored in an rstest fixture that
    // is moved into a panic_after test's thread.
    thread_handle: Mutex<Option<JoinHandle<()>>>,

}

impl DefaultSourceEncoder {
    pub fn new(keying_event_rx: BusReader<KeyingEvent>, source_encoder_tx: Bus<SourceEncoding>, terminate: Arc<AtomicBool>) -> Self {
        let builder: Box<dyn SourceEncodingBuilder + Send + Sync> = Box::new
            (BitvecSourceEncodingBuilder::new());
        let storage = Arc::new(builder);

        let arc_terminate = terminate.clone();
        let arc_storage = storage.clone();
        let thread_handle = thread::spawn(move || {
            let mut keyer_thread = EncoderKeyerThread::new(keying_event_rx,
                                                           arc_storage, arc_terminate);
            keyer_thread.thread_runner();
        });


        Self {
            keyer_speed: 12,
            source_encoder_tx,
            terminate,
            storage,
            thread_handle: Mutex::new(Some(thread_handle)),

        }
    }
}

struct EncoderKeyerThread {
    // Terminate flag
    terminate: Arc<AtomicBool>,

    // Keying channel
    keying_event_tx: BusReader<KeyingEvent>,

    // Storage
    storage: Arc<Box<dyn SourceEncodingBuilder + Send + Sync>>,
}

impl EncoderKeyerThread {
    fn new(keying_event_tx: BusReader<KeyingEvent>,
           storage: Arc<Box<dyn SourceEncodingBuilder + Send + Sync>>,
           terminate: Arc<AtomicBool>
    ) -> Self {
        debug!("Constructing EncoderKeyerThread");
        Self {
            keying_event_tx,
            storage,
            terminate,        }
    }

    // Thread that handles incoming KeyingEvents and encodes them asynchronously...
    fn thread_runner(&mut self) -> () {

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
        if self.storage.size() == 0 {
            debug!("Not emitting a SourceEncoding since there's nothing to send");
            return;
        }
    }
}

#[cfg(test)]
#[path = "./source_encoder_spec.rs"]
mod source_encoder_spec;
