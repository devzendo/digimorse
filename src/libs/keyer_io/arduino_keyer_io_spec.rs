extern crate hamcrest2;

use log::{debug, warn};
use crate::libs::serial_io::serial_io::SerialIO;
use crate::libs::util::util::*;
use std::io;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::Sender;
use std::time::Duration;

// TODO why not use rstest for a keyer fixture in this module?

struct FakeSerialIO {
    playback_chars: Vec<u8>,
    playback_index: usize,

    // Returns whatever has been sent by higher levels (the keyer's command sending routine).
    recording_tx: Sender<u8>
}

impl FakeSerialIO {
    fn new(playback: Vec<u8>, recording_tx: Sender<u8>) -> Self {
        Self { playback_chars: playback, playback_index: 0, recording_tx }
    }
}


impl SerialIO for FakeSerialIO {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for n in 0..buf.len() {
            if self.playback_index == self.playback_chars.len() {
                // Simulate real serial read timeout with a little sleep here..
                std::thread::sleep(Duration::from_millis(100));
                warn!("Out of playback data at index {}", self.playback_index);
                return Err(Error::new(ErrorKind::UnexpectedEof, format!("Out of playback data at index {}", self.playback_index)));
            }
            buf[n] = self.playback_chars[self.playback_index];
            debug!("received {}", printable(buf[n]));
            self.playback_index += 1;
        }
        return Ok(buf.len())
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for n in 0..buf.len() {
            debug!("transmitted {}", printable(buf[n]));
            match self.recording_tx.send(buf[n]) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        return Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        return Ok(())
    }
}

#[cfg(test)]
mod arduino_keyer_io_spec {
    use crate::libs::keyer_io::arduino_keyer_io::arduino_keyer_io_spec::FakeSerialIO;
    use crate::libs::keyer_io::arduino_keyer_io::ArduinoKeyer;
    use crate::libs::keyer_io::keyer_io::{Keyer, KeyingEvent, KeyingTimedEvent};
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{Arc, mpsc, Mutex, RwLock};
    use log::{debug, info};
    use std::time::Duration;
    use std::{env, thread};
    use std::sync::atomic::AtomicBool;
    use std::thread::JoinHandle;
    use bus::{Bus, BusReader};
    use crate::libs::application::application::BusOutput;
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {

    }

    struct CapturingKeyingEventReceiver {
        received_keying_events: Arc<RwLock<Vec<KeyingEvent>>>,
        thread_handle: Option<JoinHandle<()>>,
    }

    impl CapturingKeyingEventReceiver {
        fn new(mut receiver: BusReader<KeyingEvent>) -> Self {
            let vec: Vec<KeyingEvent> = vec![];
            let a_vec = Arc::new(RwLock::new(vec));
            let thread_a_vec = a_vec.clone();
            let thread_handle = thread::spawn(move || {
                debug!("CapturingKeyingEventReceiver thread starting");
                loop {
                    // Any incoming commands?
                    match receiver.recv_timeout(Duration::from_millis(250)) {
                        Ok(event) => {
                            debug!("CapturingKeyingEventReceiver received {}", &event);
                            thread_a_vec.write().unwrap().push(event);
                        }
                        Err(err) => {
                            // could timeout, or be disconnected?
                            debug!("CapturingKeyingEventReceiver error: {}", err);
                            break;
                        }
                    }
                }
                debug!("CapturingKeyingEventReceiver thread finished");
            });
            Self {
                received_keying_events: a_vec,
                thread_handle: Some(thread_handle),
            }
        }

        fn get(&self) -> Vec<KeyingEvent> {
            self.received_keying_events.read().unwrap().clone()
        }
    }

    impl Drop for CapturingKeyingEventReceiver {
        fn drop(&mut self) {
            debug!("CapturingKeyingEventReceiver joining thread handle...");
            self.thread_handle.take().map(JoinHandle::join);
            debug!("CapturingKeyingEventReceiver ...joined thread handle");
        }
    }

    #[test]
    #[serial]
    fn get_version() {
        test_util::panic_after(Duration::from_secs(2), || {
            let keyer_will_send = "v\n"; // sent to the 'arduino' ie FakeSerialIO
            let keyer_will_receive = "> v1.0.0\n\n_________"; // sent back from the 'arduino' ie FakeSerialIO

            let (recording_tx, recording_rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();
            let keying_event_tx: Arc<Mutex<Bus<KeyingEvent>>> = Arc::new(Mutex::new(Bus::new(10)));
            let keying_event_rx = keying_event_tx.lock().unwrap().add_rx();
            let capture = CapturingKeyingEventReceiver::new(keying_event_rx);

            let serial_io = FakeSerialIO::new(keyer_will_receive.as_bytes().to_vec(), recording_tx);
            let terminate = Arc::new(AtomicBool::new(false));
            let mut keyer = ArduinoKeyer::new(Box::new(serial_io), terminate);
            keyer.set_output_tx(keying_event_tx);

            match keyer.get_version() {
                Ok(v) => {
                    // Keyer replied with....
                    debug!("Keyer replied with version {}", v);
                    assert_eq!(v, "v1.0.0");
                }
                Err(e) => {
                    panic!("Did not get version: {}", e);
                }
            }

            // Keyer was sent...
            let iter = recording_rx.try_iter();
            let recording: Vec<u8> = iter.collect();
            let recording_string = String::from_utf8(recording).expect("Found invalid UTF-8");
            assert_eq!(recording_string, keyer_will_send.to_string());

            let events = capture.get();
            assert_eq!(events.is_empty(), true);
        });
    }

    #[test]
    #[serial]
    fn receive_keying() {
        test_util::panic_after(Duration::from_secs(5), || {
            // at 12 wpm, a dit is 100ms, a dah is 300ms, pause between elements 100ms, between
            // letters 300ms, between words 700ms.
            const START: u8 = 0x53;
            const END: u8 = 0x45;
            const PL: u8 = 0x2b;
            const MI: u8 = 0x2d;
            let keyer_will_receive = vec![
                START,     // start of keying

                PL, 0, 100, // P
                MI, 0, 100,
                PL, 1, 44, // 300
                MI, 0, 100,
                PL, 1, 44,
                MI, 0, 100,
                PL, 0, 100,

                MI, 1, 44, // pause between letters

                PL, 0, 100, // A
                MI, 0, 100,
                PL, 1, 44,

                MI, 1, 44, // pause between letters

                PL, 0, 100, // R
                MI, 0, 100,
                PL, 1, 44,
                MI, 0, 100,
                PL, 0, 100,

                MI, 1, 44, // pause between letters

                PL, 0, 100, // I
                MI, 0, 100,
                PL, 0, 100,

                MI, 1, 44, // pause between letters

                PL, 0, 100, // S
                MI, 0, 100,
                PL, 0, 100,
                MI, 0, 100,
                PL, 0, 100,

                END,       // end of keying
            ];
            let expected_keying_event_count = 29;

            let (recording_tx, _recording_rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();
            let keying_event_tx: Arc<Mutex<Bus<KeyingEvent>>> = Arc::new(Mutex::new(Bus::new(10)));
            let keying_event_rx = keying_event_tx.lock().unwrap().add_rx();

            let capture = CapturingKeyingEventReceiver::new(keying_event_rx);

            let serial_io = FakeSerialIO::new(keyer_will_receive, recording_tx);
            let terminate = Arc::new(AtomicBool::new(false));
            let mut keyer = ArduinoKeyer::new(Box::new(serial_io), terminate);
            keyer.set_output_tx(keying_event_tx);

            info!("Waiting for for keying...");
            thread::sleep(Duration::from_secs(3));
            info!("Out of keying wait loop");

            let received_keying_events = capture.get();
            assert_eq!(received_keying_events.len(), expected_keying_event_count);

            assert_eq!(received_keying_events[0], KeyingEvent::Start());

            // TODO these up/down polarities are wrong.
            assert_eq!(received_keying_events[1], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));
            assert_eq!(received_keying_events[2], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[3], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 300 }));
            assert_eq!(received_keying_events[4], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[5], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 300 }));
            assert_eq!(received_keying_events[6], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[7], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));

            assert_eq!(received_keying_events[8], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }));

            assert_eq!(received_keying_events[9], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));
            assert_eq!(received_keying_events[10], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[11], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 300 }));

            assert_eq!(received_keying_events[12], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }));

            assert_eq!(received_keying_events[13], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));
            assert_eq!(received_keying_events[14], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[15], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 300 }));
            assert_eq!(received_keying_events[16], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[17], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));

            assert_eq!(received_keying_events[18], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }));

            assert_eq!(received_keying_events[19], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));
            assert_eq!(received_keying_events[20], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[21], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));

            assert_eq!(received_keying_events[22], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }));

            assert_eq!(received_keying_events[23], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));
            assert_eq!(received_keying_events[24], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[25], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));
            assert_eq!(received_keying_events[26], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }));
            assert_eq!(received_keying_events[27], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }));

            assert_eq!(received_keying_events[28], KeyingEvent::End());
        });
    }

    #[test]
    #[serial]
    fn ignore_comment() {
        test_util::panic_after(Duration::from_secs(4), || {
            const START: u8 = 0x53;
            let keyer_will_receive = vec![
                0x23,     // #
                0x31,     // 1
                0x32,     // 2
                0x0A,     // /n
                START,     // start of keying
            ];
            let expected_keying_event_count = 1;

            let (recording_tx, _recording_rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();
            let keying_event_tx: Arc<Mutex<Bus<KeyingEvent>>> = Arc::new(Mutex::new(Bus::new(10)));
            let keying_event_rx = keying_event_tx.lock().unwrap().add_rx();

            let capture = CapturingKeyingEventReceiver::new(keying_event_rx);

            let serial_io = FakeSerialIO::new(keyer_will_receive, recording_tx);
            let terminate = Arc::new(AtomicBool::new(false));
            let mut keyer = ArduinoKeyer::new(Box::new(serial_io), terminate);
            keyer.set_output_tx(keying_event_tx);

            info!("Waiting for keying...");
            thread::sleep(Duration::from_secs(2));
            info!("Out of keying wait loop");

            let received_keying_events = capture.get();
            assert_eq!(received_keying_events.len(), expected_keying_event_count);

            assert_eq!(received_keying_events[0], KeyingEvent::Start());
        });
    }

    #[test]
    #[serial]
    fn terminate() {
        test_util::panic_after(Duration::from_secs(4), || {
            let keyer_will_receive = "_________"; // cause the keyer to delay its thread a bit

            let (recording_tx, _recording_rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();
            let keying_event_tx: Arc<Mutex<Bus<KeyingEvent>>> = Arc::new(Mutex::new(Bus::new(10)));
            let keying_event_rx = keying_event_tx.lock().unwrap().add_rx();
            let capture = CapturingKeyingEventReceiver::new(keying_event_rx);

            let serial_io = FakeSerialIO::new(keyer_will_receive.as_bytes().to_vec(), recording_tx);
            let terminate = Arc::new(AtomicBool::new(false));
            let mut keyer = ArduinoKeyer::new(Box::new(serial_io), terminate);
            keyer.set_output_tx(keying_event_tx);

            info!("Test sleeping");
            thread::sleep(Duration::from_millis(5)); // give things time to start
            assert_eq!(keyer.terminated(), false);
            info!("Test terminating keyer");
            keyer.terminate();
            thread::sleep(Duration::from_millis(5)); // give things time to end
            assert_eq!(keyer.terminated(), true);

            let events = capture.get();
            assert_eq!(events.is_empty(), true);
        });
    }
}
