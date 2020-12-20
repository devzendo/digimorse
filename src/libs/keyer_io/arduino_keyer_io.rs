use log::{warn, info, debug};

use crate::libs::keyer_io::keyer_io::{Keyer, KeyingEdgeEventListener, KeyerPolarity, KeyingMode, KeyerOutputMode};
use crate::libs::serial_io::serial_io::SerialIO;
use std::io::Error;
use crate::libs::util::util::printable;

pub struct ArduinoKeyer<'a> {
    serial_io:&'a mut (SerialIO + 'a),
    // thread waiting for data, or async read/promise
    // current callback to receive non-pulse/timing data (lines starting with > until blank NL)
}

impl<'a> ArduinoKeyer<'a> {
    fn new(s: &'a mut SerialIO) -> Self {
        Self {
            serial_io: s
        }
    }
/*    fn transact(&self, command: String) -> Result<String, String> {
        // set callback to build up
        // send(command);
    }
*/
}
impl<'a> Keyer for ArduinoKeyer<'a> {
    fn get_version(&mut self) -> Result<String, String> {
        let keyer_will_send = "v\n";
        let written_bytes = self.serial_io.write(keyer_will_send.as_bytes());
        match written_bytes {
            Ok(n) => {
                debug!("Written {} bytes to keyer", n);
                let mut read_text: Vec<u8> = vec![];
                let mut read_buf: [u8; 1] = [0];
                let mut start_of_line: bool = true;
                loop {
                    let read_bytes = self.serial_io.read(&mut read_buf);
                    match read_bytes {
                        Ok(1) => {
                            debug!("transact read {}", printable(read_buf[0]));
                            if read_buf[0] == 0x0a {
                                debug!("Got NL...");
                                if start_of_line {
                                    debug!("NL read on its own: end of response");
                                    let mut subslice = &read_text[2..read_text.len() - 1];

                                    return Ok(std::str::from_utf8(subslice).expect("Found invalid UTF-8").parse().unwrap());
                                }
                                start_of_line = true;
                            } else {
                                debug!("Got non-NL...");
                                start_of_line = false;
                            }
                            read_text.push(read_buf[0]);
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


#[cfg(test)]
#[path = "./arduino_keyer_io_spec.rs"]
mod arduino_keyer_io_spec;
