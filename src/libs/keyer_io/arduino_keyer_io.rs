use log::{warn, info, debug};

use crate::libs::keyer_io::keyer_io::{Keyer, KeyingEdgeEventListener, KeyerPolarity, KeyingMode, KeyerOutputMode};
use crate::libs::serial_io::serial_io::SerialIO;
use std::io::Error;
use crate::libs::util::util::printable;
use crate::libs::keyer_io::arduino_keyer_io::KeyerState::{Initial, ResponseGotGt, ResponseGotSpc, ResponseFinish};

#[derive(Debug)]
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
    start_of_line: bool,
}

impl<'a> ArduinoKeyer<'a> {
    fn new(s: &'a mut SerialIO) -> Self {
        Self {
            serial_io: s,
            state: Initial,
            read_text: vec![],
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
                self.set_state(Initial);
                let mut read_buf: [u8; 1] = [0];

                loop {
                    let read_bytes = self.serial_io.read(&mut read_buf);
                    match read_bytes {
                        Ok(1) => {
                            debug!("transact read {}", printable(read_buf[0]));
                            let next: Option<Result<String, String>> = match self.state {
                                KeyerState::Initial => {
                                    self.initial(read_buf[0])
                                }
                                KeyerState::KeyingDurationGetLSB => {
                                    self.keying_duration_get_lsb(read_buf[0])
                                }
                                KeyerState::KeyingDurationGetMSB => {
                                    self.keying_duration_get_msb(read_buf[0])
                                }
                                KeyerState::ResponseGotGt => {
                                    self.response_got_gt(read_buf[0])
                                }
                                KeyerState::ResponseGotSpc => {
                                    self.response_got_spc(read_buf[0])
                                }
                                KeyerState::ResponseFinish => {
                                    self.response_finish(read_buf[0])
                                }
                            };
                            match next {
                                // A return of some type is needed
                                Some(result) => {
                                    return result;
                                }
                                // State may have changed, stay in here, read more...
                                None => {}
                            }
/*                            if read_buf[0] == 0x0a {
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
                            self.read_text.push(read_buf[0]);

 */
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

    fn set_state(&mut self, new_state: KeyerState) {
        debug!("Changing state to {:?}", new_state);
        self.state = new_state;
    }

    fn initial(&mut self, ch: u8) -> Option<Result<String, String>> {
        match ch {
            b'>' => {
                self.read_text.clear();
                self.set_state(ResponseGotGt);
            }
            // TODO S
            // TODO E
            // TODO -
            // TODO +
            _ => {
                warn!("Unexpected out-of-state data {}", printable(ch));
            }
        }
        None
    }

    fn keying_duration_get_lsb(&mut self, ch: u8) -> Option<Result<String, String>> {
        None
    }

    fn keying_duration_get_msb(&mut self, ch: u8) -> Option<Result<String, String>> {
        None
    }

    fn response_got_gt(&mut self, ch: u8) -> Option<Result<String, String>> {
        return match ch {
            b' ' => {
                self.set_state(ResponseGotSpc);
                None
            }
            _ => {
                warn!("Unexpected response data {}", printable(ch));
                Some(Err(format!("Unexpected response data {}", printable(ch))))
            }
        }
    }

    fn response_got_spc(&mut self, ch: u8) -> Option<Result<String, String>> {
        match ch {
            b'\n' => {
                // maybe... self.read_text.push(ch);
                self.set_state(ResponseFinish);
            }
            _ => {
                self.read_text.push(ch);
            }
        }
        None
    }

    fn response_finish(&mut self, ch: u8) -> Option<Result<String, String>> {
        return match ch {
            b'>' => {
                self.set_state(ResponseGotGt);
                None
            }
            b'\n' => {
                self.set_state(Initial);
                let mut subslice = &self.read_text[0..self.read_text.len()];
                Some(Ok(String::from_utf8(Vec::from(subslice)).expect("Found invalid UTF-8")))
            }
            _ => {
                warn!("Unexpected response data {}", printable(ch));
                Some(Err(format!("Unexpected response data {}", printable(ch))))
            }
        }
    }

}


#[cfg(test)]
#[path = "./arduino_keyer_io_spec.rs"]
mod arduino_keyer_io_spec;
