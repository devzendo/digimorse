extern crate hamcrest2;

use log::{debug};
use crate::libs::serial_io::serial_io::SerialIO;
use std::io;

struct FakeSerialIO {
    playback_chars: Vec<u8>,
    playback_index: usize,
    record_chars: Vec<u8>
}

impl FakeSerialIO {
    fn new(playback: String) -> Self {
        Self { playback_chars: playback.into_bytes(), playback_index: 0, record_chars: vec![] }
    }

    // Returns whatever has been sent by higher levels (the keyer's command sending routine).
    fn recording(&self) -> String {
        return String::from_utf8(Vec::from(self.record_chars.as_slice())).expect("Found invalid UTF-8")
    }
}

impl SerialIO for FakeSerialIO {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for n in 0..buf.len() {
            buf[n] = self.playback_chars[self.playback_index + 1];
            debug!("received {}", buf[n]);
            self.playback_index += 1;
        }
        return Ok(buf.len())
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for n in 0..buf.len() {
            debug!("transmitted {}", buf[n]);
            self.record_chars.push(buf[n]);
        }
        return Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        return Ok(())
    }
}

#[cfg(test)]
mod arduino_keyer_io_spec {
    //use hamcrest2::prelude::*;
    use crate::libs::keyer_io::arduino_keyer_io::arduino_keyer_io_spec::FakeSerialIO;
    use crate::libs::keyer_io::arduino_keyer_io::ArduinoKeyer;
    use crate::libs::keyer_io::keyer_io::Keyer;
    use crate::libs::serial_io::serial_io::SerialIO;

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

        let serial_io = FakeSerialIO::new(keyer_will_receive.to_string());
        let keyer = ArduinoKeyer::new(&serial_io as &SerialIO);
        match keyer.get_version() {
            Ok(v) => {
                assert_eq!(v, "v1.0.0");
            }
            Err(e) => {
                panic!("Did not get version: {}", e);
            }
        }
        assert_eq!(serial_io.recording(), keyer_will_send.to_string());
    }
}
