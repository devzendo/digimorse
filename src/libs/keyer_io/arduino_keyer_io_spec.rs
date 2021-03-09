extern crate hamcrest2;

use log::{debug, warn};
use crate::libs::serial_io::serial_io::SerialIO;
use crate::libs::util::util::*;
use std::io;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{Sender};

struct FakeSerialIO {
    playback_chars: Vec<u8>,
    playback_index: usize,

    // Returns whatever has been sent by higher levels (the keyer's command sending routine).
    recording_tx: Sender<u8>
}

impl FakeSerialIO {
    fn new(playback: String, recording_tx: Sender<u8>) -> Self {
        Self { playback_chars: playback.into_bytes(), playback_index: 0, recording_tx }
    }
}


impl SerialIO for FakeSerialIO {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for n in 0..buf.len() {
            if self.playback_index == self.playback_chars.len() {
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
    use crate::libs::keyer_io::keyer_io::Keyer;
    use std::sync::mpsc::{Sender, Receiver};
    use std::sync::mpsc;

    #[ctor::ctor]
    fn before_each() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {

    }

    #[test]
    fn get_version() {
        let keyer_will_send = "v\n";
        let keyer_will_receive = "> v1.0.0\n\n";

        let (recording_tx, recording_rx): (Sender<u8>, Receiver<u8>) = mpsc::channel();

        let serial_io = FakeSerialIO::new(keyer_will_receive.to_string(), recording_tx);
        let mut keyer = ArduinoKeyer::new(Box::new(serial_io));
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
    }
}
