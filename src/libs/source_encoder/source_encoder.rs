use std::sync::{Arc, mpsc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use bus::{Bus, BusReader};
use log::{debug, info};
use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed, KeyingTimedEvent};
use crate::libs::source_encoder::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
use crate::libs::source_encoder::keying_encoder::{DefaultKeyingEncoder, KeyingEncoder};
use crate::libs::source_encoder::source_encoding::{EncoderFrameType, SourceEncoding, SourceEncodingBuilder};

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

struct Emitter {
    storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>, // ?? Is it Send + Sync?
    keying_encoder: Box<dyn KeyingEncoder + Send + Sync>,
    source_encoder_tx: Bus<SourceEncoding>,
    sent_wpm_polarity: bool,
    keying_speed: KeyerSpeed,
}

impl Emitter {
    fn emit(&mut self) {
        if self.storage.read().unwrap().size() == 0 {
            debug!("Not emitting a SourceEncoding since there's nothing to send");
            return;
        }
        let encoding = self.storage.write().unwrap().build();
        debug!("Emitting {}", encoding);
        self.source_encoder_tx.broadcast(encoding);
    }

    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keying_speed = speed;
        self.keying_encoder.set_keyer_speed(speed);
    }

    fn keying_event(&mut self, keying_event: KeyingEvent) {
        match keying_event {
            KeyingEvent::Start() => {
                // Don't add anything to storage, but should reset the polarity to Mark
                // TODO needs test
            }
            KeyingEvent::Timed(timed) => {
                if !self.sent_wpm_polarity {
                    self.sent_wpm_polarity = true;
                    // TODO need to encode the WPM/Polarity
                    let mut storage = self.storage.write().unwrap();
                    let frame_type = EncoderFrameType::WPMPolarity;
                    debug!("Adding {:?} {} WPM, polarity {} ", frame_type, self
                                    .keying_speed, if true { "MARK" } else { "SPACE" }); // TODO
                    // see below..
                    storage.add_8_bits(frame_type as u8, 4);
                    storage.add_8_bits(self.keying_speed, 6);
                    storage.add_bool(true); // TODO needs test for first keying event
                    // of 2nd block being space.
                    // TODO what if there's no room?
                }
                // TODO pass on to CQ detector
                self.keying_encoder.encode_keying(timed);
                // TODO what if this returns false? means that the keying won't fit
                // so we must build() and broadcast the builder's vec, then try again.
            }
            KeyingEvent::End() => {
                // Set the end of the storage
                // TODO needs test
            }
        }
    }
}

#[readonly::make]
pub struct DefaultSourceEncoder {
    keyer_speed: KeyerSpeed,
    terminate: Arc<AtomicBool>,
    storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>, // ?? Is it Send + Sync?
    // Send + Sync are here so the DefaultSourceEncoder can be stored in an rstest fixture that
    // is moved into a panic_after test's thread.
    thread_handle: Mutex<Option<JoinHandle<()>>>,
    emitter: Arc<Mutex<Emitter>>,
}

impl DefaultSourceEncoder {
    pub fn new(keying_event_rx: BusReader<KeyingEvent>, source_encoder_tx: Bus<SourceEncoding>, terminate: Arc<AtomicBool>) -> Self {
        let builder: Box<dyn SourceEncodingBuilder + Send + Sync> = Box::new
            (BitvecSourceEncodingBuilder::new());
        let storage = Arc::new(RwLock::new(builder));
        let arc_storage = storage.clone();

        let arc_terminate = terminate.clone();

        let encoder: Box<dyn KeyingEncoder + Send + Sync> = Box::new(DefaultKeyingEncoder::new
            (storage.clone()));

        let emitter = Mutex::new(Emitter {
            storage: storage.clone(),
            keying_encoder: encoder,
            source_encoder_tx,
            sent_wpm_polarity: false,
            keying_speed: 0,
        });
        let arc_emitter = Arc::new(emitter);
        let arc_emitter_cloned = arc_emitter.clone();
        let thread_handle = thread::spawn(move || {
            let mut keyer_thread = EncoderKeyerThread::new(keying_event_rx,
                                                            arc_terminate,
                                                           arc_emitter.clone());
            keyer_thread.thread_runner();
        });

        Self {
            keyer_speed: 12,
            terminate,
            storage: arc_storage,
            thread_handle: Mutex::new(Some(thread_handle)),
            emitter: arc_emitter_cloned,
        }
    }

    // Signals the thread to terminate, blocks on joining the handle. Used by drop().
    // Setting the terminate AtomicBool will allow the thread to stop on its own, but there's no
    // method other than this for blocking until it has actually stopped.
    pub fn terminate(&mut self) {
        debug!("Terminating encoder");
        self.terminate.store(true, Ordering::SeqCst);
        debug!("DefaultSourceEncoder joining thread handle...");
        let mut thread_handle = self.thread_handle.lock().unwrap();
        thread_handle.take().map(JoinHandle::join);
        debug!("DefaultSourceEncoder ...joined thread handle");
    }

    // Has the thread finished (ie has it been joined)?
    pub fn terminated(&mut self) -> bool {
        debug!("Is encoder terminated?");
        let ret = self.thread_handle.lock().unwrap().is_none();
        debug!("Termination state is {}", ret);
        ret
    }

}

impl Drop for DefaultSourceEncoder {
    fn drop(&mut self) {
        debug!("DefaultSourceEncoder signalling termination to thread on drop");
        self.terminate();
    }
}

impl SourceEncoder for DefaultSourceEncoder {
    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keyer_speed = speed;
        // TODO pass on to CQ detector
        self.emitter.lock().unwrap().set_keyer_speed(speed);
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }

    fn emit(&mut self) {
        self.emitter.lock().unwrap().emit();
    }
}


struct EncoderKeyerThread {
    // Terminate flag
    terminate: Arc<AtomicBool>,

    // Incoming Keying channel
    keying_event_tx: BusReader<KeyingEvent>,

    // Shared state between thread and main code
    emitter: Arc<Mutex<Emitter>>,
}

impl EncoderKeyerThread {
    fn new(keying_event_tx: BusReader<KeyingEvent>,
           terminate: Arc<AtomicBool>,
           emitter: Arc<Mutex<Emitter>>
    ) -> Self {
        debug!("Constructing EncoderKeyerThread");
        Self {
            terminate,
            keying_event_tx,
            emitter,
        }
    }

    // Thread that handles incoming KeyingEvents and encodes them asynchronously...
    fn thread_runner(&mut self) -> () {
        info!("Encoding thread started");
        loop {
            if self.terminate.load(Ordering::SeqCst) {
                info!("Terminating encoding thread");
                break;
            }

            match self.keying_event_tx.recv_timeout(Duration::from_millis(100)) {
                Ok(keying_event) => {
                    self.emitter.lock().unwrap().keying_event(keying_event);
                }
                Err(_) => {
                    // Don't log, it's just noise - timeout gives opportunity to go round loop and
                    // check for terminate.
                }
            }
        }
        info!("Encoding thread stopped");
    }
}


#[cfg(test)]
#[path = "./source_encoder_spec.rs"]
mod source_encoder_spec;
