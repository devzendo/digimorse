use log::{warn, info, debug};

use crate::libs::keyer_io::keyer_io::{Keyer, KeyingEdgeEventListener, KeyerPolarity, KeyingMode, KeyerOutputMode};
use crate::libs::serial_io::serial_io::SerialIO;
use std::io::Error;
use crate::libs::util::util::printable;
use crate::libs::keyer_io::arduino_keyer_io::KeyerState::Initial;

pub enum KeyerState {
    Initial,
    KeyingDurationGetLSB, KeyingDurationGetMSB,
    ResponseGotGt, ResponseGotSpc, ResponseFinish
}

pub struct ArduinoKeyer<'a> {
    serial_io:&'a mut (SerialIO + 'a),
    // thread waiting for data, or async read/promise
    // current callback to receive non-pulse/timing data (lines starting with > until blank NL)
    state: KeyerState,
    read_text: Vec<u8>,
    read_buf: [u8; 1],
    start_of_line: bool,
}

impl<'a> ArduinoKeyer<'a> {
    fn new(s: &'a mut SerialIO) -> Self {
        Self {
            serial_io: s,
            state: Initial,
            read_text: vec![],
            read_buf: [0],
            start_of_line: true
        }
    }
}

impl<'a> Keyer for ArduinoKeyer<'a> {
    fn get_version(&mut self) -> Result<String, String> {
        let keyer_will_send = "v\n";

        self.transact(keyer_will_send)
    }

    fn get_speed(&mut self) -> Result<u8, String> {
        unimplemented!()
    }

    fn set_speed(&mut self, _wpm: u8) -> Result<(), String> {
        unimplemented!()
    }

    fn get_keying_mode(&mut self) -> Result<KeyingMode, String> {
        unimplemented!()
    }

    fn set_keying_mode(&mut self, _mode: KeyingMode) -> Result<(), String> {
        unimplemented!()
    }

    fn get_keyer_polarity(&mut self) -> Result<KeyerPolarity, String> {
        unimplemented!()
    }

    fn set_keyer_polarity(&mut self, _polarity: KeyerPolarity) -> Result<(), String> {
        unimplemented!()
    }

    fn get_keyer_output_mode(&mut self) -> Result<KeyerOutputMode, String> {
        unimplemented!()
    }

    fn set_keyer_output_mode(&mut self, _mode: KeyerOutputMode) -> Result<(), String> {
        unimplemented!()
    }

    fn set_edge_event_listener(&mut self, _pulse_event_listener: &mut dyn KeyingEdgeEventListener) {
        unimplemented!()
    }

    fn clear_edge_event_listener(&mut self) {
        unimplemented!()
    }
}

impl<'a> ArduinoKeyer<'a> {
    fn transact(&mut self, command_to_keyer: &str) -> Result<String, String> {
        let written_bytes = self.serial_io.write(command_to_keyer.as_bytes());
        match written_bytes {
            Ok(n) => {
                debug!("Written {} bytes to keyer", n);
                self.state = Initial;
                loop {
                    let read_bytes = self.serial_io.read(&mut self.read_buf);
                    match read_bytes {
                        Ok(1) => {
                            debug!("transact read {}", printable(self.read_buf[0]));
                            if self.read_buf[0] == 0x0a {
                                debug!("Got NL...");
                                if self.start_of_line {
                                    debug!("NL read on its own: end of response");
                                    // Given ">_XXXX\n" return "XXXX"
                                    let mut subslice = &self.read_text[2..self.read_text.len() - 1];

                                    return Ok(std::str::from_utf8(subslice).expect("Found invalid UTF-8").parse().unwrap());
                                }
                                self.start_of_line = true;
                            } else {
                                debug!("Got non-NL...");
                                self.start_of_line = false;
                            }
                            self.read_text.push(self.read_buf[0]);
                        }
                        Ok(n) => {
                            warn!("In build loop, received {} bytes", n);
                            return Err(format!("Build loop stopped reading, received {} bytes", n));
                        }
                        Err(e) => {
                            return Err(format!("Could not read data from keyer: {}", e.to_string()))
                        }
                    }
                }
            }
            Err(e) => {
                return Err(format!("Could not write command to keyer: {}", e.to_string()))
            }
        }
    }
}


#[cfg(test)]
#[path = "./arduino_keyer_io_spec.rs"]
mod arduino_keyer_io_spec;
