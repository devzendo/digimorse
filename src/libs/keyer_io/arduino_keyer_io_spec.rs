extern crate hamcrest2;

use log::{debug, warn};
use crate::libs::serial_io::serial_io::SerialIO;
use crate::libs::util::util::*;
use std::io;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{Sender};
use std::time::Duration;

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
                std::thread::sleep(Duration::from_millis(50));
                warn!("Out of playback data at index {}", self.playback_index);
                return Err(Error::new(ErrorKind::Other, format!("Out of playback data at index {}", self.playback_index)));
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
    use std::sync::mpsc::{Sender, Receiver};
    use std::sync::{mpsc, Arc};
    use log::info;
    use std::time::Duration;
    use std::env;
    use actix::{Actor, Context, Handler, AsyncContext, Addr};
    use parking::{Parker, Unparker};

    struct CapturingKeyingEventReceiver {
        received_keying_events: Vec<KeyingEvent>,
        addr: Option<Addr<CapturingKeyingEventReceiver>>,
        parker: Parker,
        unparker: Unparker,
    }
    impl CapturingKeyingEventReceiver {
        fn new() -> Self {
            let parker = Parker::new();
            let unparker = parker.unparker();

            Self {
                received_keying_events: vec![],
                addr: None,
                parker,
                unparker,
            }
        }
        fn get(&self) -> Arc<Vec<KeyingEvent>> {
            let cloned = self.received_keying_events.clone();
            Arc::new(cloned)
        }
        fn get_addr(&self) -> Addr<CapturingKeyingEventReceiver> {
            info!("Waiting for addr to be set");
            self.parker.park();
            info!("Addr is available");
            self.addr.clone().unwrap()
        }
    }
    impl Actor for CapturingKeyingEventReceiver {
        type Context = Context<Self>;

        fn started(&mut self, ctx: &mut Self::Context) {
            info!("Actor started");
            self.addr = Some(ctx.address());
            info!("Addr is set, unparking");
            self.unparker.unpark();
            info!("Unparked; end of started");
        }
        fn stopped(&mut self, ctx: &mut Self::Context) {
            info!("Actor stopped");
        }

    }
    impl Handler<KeyingEvent> for CapturingKeyingEventReceiver {
        type Result = ();
        fn handle(&mut self, keying_event: KeyingEvent, _ctx: &mut Self::Context) {
            info!("Actor received KeyingEvent {}", keying_event);
            self.received_keying_events.push(keying_event);
        }
    }

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {
    }

    #[test]
    fn get_version() {
        let keyer_will_send = "v\n"; // sent to the 'arduino' ie FakeSerialIO
        let keyer_will_receive = "> v1.0.0\n\n"; // sent back from the 'arduino' ie FakeSerialIO

        info!("Initialising actix System...");
        let system = actix::System::new();
        info!("... initialised.");

        let capturing_keying_event_receiver = CapturingKeyingEventReceiver::new();
        info!("Created actor");
        let events = capturing_keying_event_receiver.get();
        info!("Got events");

        info!("Running System...");
        system.block_on(async {
            capturing_keying_event_receiver.start()
        });
        info!("Block on start finished");
        let capturing_keying_event_receiver_addr = capturing_keying_event_receiver.get_addr();
        info!("Started actor and got addr");
        let recipient_keying_event = capturing_keying_event_receiver_addr.recipient();
        info!("Got recipient");


        let (recording_tx, recording_rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();

        let serial_io = FakeSerialIO::new(keyer_will_receive.as_bytes().to_vec(), recording_tx);
        let mut keyer = ArduinoKeyer::new(Box::new(serial_io), recipient_keying_event);

        match keyer.get_version() {
            Ok(v) => {
                // Keyer replied with....
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

        assert_eq!(events.len(), 0);

        info!("End of test.");
    }

    #[test]
    fn receive_keying() {
        // at 12 wpm, a dit is 10ms, a dah is 30ms, pause between elements 10ms, between letters
        // 30ms, between words 70ms.
        const START: u8 = 0x53;
        const END: u8 = 0x45;
        const PL: u8 = 0x2b;
        const MI: u8 = 0x2d;
        let keyer_will_receive = vec![
            START,     // start of keying

            PL, 0, 10, // P
            MI, 0, 10,
            PL, 0, 30,
            MI, 0, 10,
            PL, 0, 30,
            MI, 0, 10,
            PL, 0, 10,

            MI, 0, 30, // pause between letters
   
            PL, 0, 10, // A
            MI, 0, 10,
            PL, 0, 30,
   
            MI, 0, 30, // pause between letters
   
            PL, 0, 10, // R
            MI, 0, 10,
            PL, 0, 30,
            MI, 0, 10,
            PL, 0, 10,
   
            MI, 0, 30, // pause between letters
   
            PL, 0, 10, // I
            MI, 0, 10,
            PL, 0, 10,
   
            MI, 0, 30, // pause between letters
   
            PL, 0, 10, // S
            MI, 0, 10,
            PL, 0, 10,
            MI, 0, 10,
            PL, 0, 10,

            END,       // end of keying
        ];
        let expected_keying_event_count = 29;

        info!("Initialising actix System...");
        let system = actix::System::new();
        info!("... initialised.");

        let capturing_keying_event_receiver = CapturingKeyingEventReceiver::new();
        info!("Created actor");
        let received_keying_events= capturing_keying_event_receiver.get();
        info!("Got events");
        let arc_actor = Arc::new(capturing_keying_event_receiver);
        info!("Running System...");
        system.block_on(async {
            arc_actor.start();
        });
        let capturing_keying_event_receiver_addr = capturing_keying_event_receiver.start();
        info!("Started actor and got addr");
        let recipient_keying_event = capturing_keying_event_receiver_addr.recipient();
        info!("Got recipient");

        let (recording_tx, _recording_rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();

        let serial_io = FakeSerialIO::new(keyer_will_receive, recording_tx);
        let _keyer = ArduinoKeyer::new(Box::new(serial_io), recipient_keying_event);

        info!("Waiting for keying...");
        //tokio::time::sleep(Duration::from_millis(3000)).await;
        std::thread::sleep(Duration::from_secs(3));

/*            assert_eq!(received_keying_events.len(), expected_keying_event_count);

        assert_eq!(received_keying_events[0], KeyingEvent::Start());

        assert_eq!(received_keying_events[1], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));
        assert_eq!(received_keying_events[2], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[3], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 30 }));
        assert_eq!(received_keying_events[4], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[5], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 30 }));
        assert_eq!(received_keying_events[6], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[7], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));

        assert_eq!(received_keying_events[8], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 30 }));

        assert_eq!(received_keying_events[9], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));
        assert_eq!(received_keying_events[10], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[11], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 30 }));

        assert_eq!(received_keying_events[12], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 30 }));

        assert_eq!(received_keying_events[13], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));
        assert_eq!(received_keying_events[14], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[15], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 30 }));
        assert_eq!(received_keying_events[16], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[17], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));

        assert_eq!(received_keying_events[18], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 30 }));

        assert_eq!(received_keying_events[19], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));
        assert_eq!(received_keying_events[20], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[21], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));

        assert_eq!(received_keying_events[22], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 30 }));

        assert_eq!(received_keying_events[23], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));
        assert_eq!(received_keying_events[24], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[25], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));
        assert_eq!(received_keying_events[26], KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }));
        assert_eq!(received_keying_events[27], KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }));

        assert_eq!(received_keying_events[28], KeyingEvent::End());
        */


    }

    #[test]
    fn ignore_comment() {
        const START: u8 = 0x53;
        let keyer_will_receive = vec![
            0x23,     // #
            0x31,     // 1
            0x32,     // 2
            0x0A,     // /n
            START,     // start of keying
        ];
        let expected_keying_event_count = 1;

        let capturing_keying_event_receiver = CapturingKeyingEventReceiver::new();
        let received_keying_events = capturing_keying_event_receiver.get();
        let capturing_keying_event_receiver_addr = capturing_keying_event_receiver.start();
        let recipient_keying_event = capturing_keying_event_receiver_addr.recipient();

        let (recording_tx, _recording_rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();

        let serial_io = FakeSerialIO::new(keyer_will_receive, recording_tx);
        let _keyer = ArduinoKeyer::new(Box::new(serial_io), recipient_keying_event);

        info!("Waiting for keying...");
        std::thread::sleep(Duration::from_millis(3000));

        assert_eq!(received_keying_events.len(), expected_keying_event_count);

        assert_eq!(received_keying_events[0], KeyingEvent::Start());
    }
}
