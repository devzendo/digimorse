use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{mem, thread};
use std::thread::JoinHandle;
use std::time::Duration;
use bus::{Bus, BusReader};
use log::{debug, info};
use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed};
use crate::libs::source_codec::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
use crate::libs::source_codec::keying_encoder::{DefaultKeyingEncoder, KeyingEncoder};
use crate::libs::source_codec::source_encoding::{EncoderFrameType, SourceEncoding, SourceEncodingBuilder};

/*
 * The source encoder transforms keying information into a number of frames.
 * Batch up frames to some maximum block size, yet to be determined. When full, emit them as a
 * Vec<u8>.
 * Track up/down key state, send the current state as the first WPM/Polarity encoding in a block so
 * that the receiver knows whether it's mark or space, per-block. If a block does not decode
 * correctly the receiver will miss that block and need to re-sync its idea of mark/space.
 *
 * If we're aiming for a wide range of WPM speeds, 5 to 60WPM, these have a wide range of dit/dah
 * element durations in ms.
 * A dit at 60WPM is 20ms (and it could be sent short).
 * A wordgap at 5WPM is 1680ms (and it could be sent long).
 * So a range of 20ms to 1680ms.
 * If the keying is 'perfect' (or very close to it), encode it optimally.
 * If it's within the usual deltas for the current speed, encode it as a +/- delta from ideal using
 * the least number of bits that can contain such a delta (bearing in mind that the +/-ve delta
 * ranges change from high to low as the speed changes from low to high). If it is not within the
 * usual deltas, encode it naïvely.
 * Also inject metadata frames as needed - after a given time, and if <START>CQ is detected.
 */

#[readonly::make]
pub struct SourceEncoder {
    keyer_speed: KeyerSpeed,
    terminate: Arc<AtomicBool>,
    storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>, // ?? Is it Send + Sync?
    // Send + Sync are here so the DefaultSourceEncoder can be stored in an rstest fixture that
    // is moved into a panic_after test's thread.
    thread_handle: Mutex<Option<JoinHandle<()>>>,
    shared: Arc<Mutex<SourceEncoderShared>>,
}

impl SourceEncoder {
    pub fn new(keying_event_rx: BusReader<KeyingEvent>, source_encoder_tx: Bus<SourceEncoding>, terminate: Arc<AtomicBool>) -> Self {
        let builder: Box<dyn SourceEncodingBuilder + Send + Sync> = Box::new
            (BitvecSourceEncodingBuilder::new());
        let arc_storage = Arc::new(RwLock::new(builder));
        let arc_storage_cloned = arc_storage.clone();

        let arc_terminate = terminate.clone();

        let encoder: Box<dyn KeyingEncoder + Send + Sync> = Box::new(DefaultKeyingEncoder::new
            (arc_storage.clone()));

        let shared = Mutex::new(SourceEncoderShared {
            storage: arc_storage.clone(),
            keying_encoder: encoder,
            source_encoder_tx,
            is_mark: true,
            sent_wpm_polarity: false,
            keying_speed: 0,
        });
        let arc_shared = Arc::new(shared);
        let arc_shared_cloned = arc_shared.clone();
        let thread_handle = thread::spawn(move || {
            let mut keyer_thread = SourceEncoderKeyerThread::new(keying_event_rx,
                                                                 arc_terminate,
                                                                 arc_shared.clone());
            keyer_thread.thread_runner();
        });

        Self {
            keyer_speed: 12,
            terminate,
            storage: arc_storage_cloned,
            thread_handle: Mutex::new(Some(thread_handle)),
            shared: arc_shared_cloned,
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

    // The SourceEncoder needs to know the keyer speed to build keying frames into their most
    // compact form; a minimal delta from the three timing elements.
    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keyer_speed = speed;
        // TODO pass on to CQ detector
        self.shared.lock().unwrap().set_keyer_speed(speed);
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }

    // Irrespective of how full the current frame is, pad it to SOURCE_ENCODER_BLOCK_SIZE and emit
    // it on the output Bus<SourceEncoding>.
    fn emit(&mut self) {
        self.shared.lock().unwrap().emit();
    }
}

impl Drop for SourceEncoder {
    fn drop(&mut self) {
        debug!("DefaultSourceEncoder signalling termination to thread on drop");
        self.terminate();
    }
}


// An object shared between the main SourceEncoder, and the SourceEncoderKeyerThread -
// the KeyingEvent handling thread.
struct SourceEncoderShared {
    storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>, // ?? Is it Send + Sync?
    keying_encoder: Box<dyn KeyingEncoder + Send + Sync>,
    source_encoder_tx: Bus<SourceEncoding>,
    sent_wpm_polarity: bool,
    is_mark: bool,
    keying_speed: KeyerSpeed,
}

impl SourceEncoderShared {
    fn emit(&mut self) {
        if self.storage.read().unwrap().size() == 0 {
            debug!("Not emitting a SourceEncoding since there's nothing to send");
            return;
        }
        self.sent_wpm_polarity = false;
        let encoding = self.storage.write().unwrap().build();
        debug!("Emitting {}", encoding);
        self.source_encoder_tx.broadcast(encoding);
    }

    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keying_speed = speed;
        self.keying_encoder.set_keyer_speed(speed);
        // Ensure WPM|Polarity is sent before the next Keying.
        self.sent_wpm_polarity = false;
    }

    fn keying_event(&mut self, keying_event: KeyingEvent) {
        debug!("Encoding keying event {}", keying_event);
        match keying_event {
            KeyingEvent::Start() => {
                // Don't add anything to storage, but reset the polarity to Mark
                self.is_mark = true;
            }
            KeyingEvent::Timed(timed) => {
                if !self.sent_wpm_polarity {
                    self.sent_wpm_polarity = true;
                    loop {
                        let mut storage = self.storage.write().unwrap();
                        let remaining = storage.remaining();
                        if remaining < 11 {
                            mem::drop(storage);
                            debug!("Insufficient space ({}) to encode WPM|Polarity", remaining);
                            self.emit();
                        } else {
                            let frame_type = EncoderFrameType::WPMPolarity;
                            debug!("Adding {:?} {} WPM, polarity {} ", frame_type, self
                                            .keying_speed, if timed.up { "MARK" } else { "SPACE" }); //
                            storage.add_8_bits(frame_type as u8, 4);
                            storage.add_8_bits(self.keying_speed, 6);
                            storage.add_bool(self.is_mark);
                            mem::drop(storage);
                            break;
                        }
                    }
                }
                // TODO pass on to CQ detector
                self.keying_encoder.encode_keying(timed);
                self.is_mark = !self.is_mark;
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


struct SourceEncoderKeyerThread {
    // Terminate flag
    terminate: Arc<AtomicBool>,

    // Incoming Keying channel
    keying_event_tx: BusReader<KeyingEvent>,

    // Shared state between thread and main code
    shared: Arc<Mutex<SourceEncoderShared>>,
}

impl SourceEncoderKeyerThread {
    fn new(keying_event_tx: BusReader<KeyingEvent>,
           terminate: Arc<AtomicBool>,
           shared: Arc<Mutex<SourceEncoderShared>>
    ) -> Self {
        debug!("Constructing EncoderKeyerThread");
        Self {
            terminate,
            keying_event_tx,
            shared,
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
                    self.shared.lock().unwrap().keying_event(keying_event);
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
