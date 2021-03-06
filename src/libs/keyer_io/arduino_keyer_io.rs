use log::{warn, debug};

use crate::libs::keyer_io::arduino_keyer_io::KeyerState::{Initial, ResponseGotGt, ResponseGotSpc, ResponseFinish, KeyingDurationGetLSB, KeyingDurationGetMSB};
use crate::libs::keyer_io::keyer_io::{Keyer, KeyerPolarity, KeyingMode, KeyingEvent, KeyerEdgeDurationMs, KeyingTimedEvent};
use crate::libs::serial_io::serial_io::SerialIO;
use crate::libs::util::util::printable;
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use crate::libs::keyer_io::keyer_io::KeyingEvent::{Timed, Start, End};

pub struct ArduinoKeyer {
    // Command channel to/from the thread. Sender is guarded by a Mutex to ensure a single command
    // in flight at a time.
    command_request_tx: Mutex<Sender<String>>,
    command_response_rx: Receiver<Result<String, String>>,

    _thread_handle: JoinHandle<()>,
}

impl ArduinoKeyer {
    fn new(serial_io: Box<dyn SerialIO>, keying_event_tx: Sender<KeyingEvent>) -> Self {
        // Channels have two endpoints: the `Sender<T>` and the `Receiver<T>`,
        // where `T` is the type of the message to be transferred
        // (type annotation is superfluous)
        let (command_request_tx, command_request_rx): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (command_response_tx, command_response_rx): (Sender<Result<String, String>>, Receiver<Result<String, String>>) = mpsc::channel();
        let mutex_command_request_tx = Mutex::new(command_request_tx);

        let thread_handle = thread::spawn(move || {
            let mut arduino_keyer_thread = ArduinoKeyerThread::new(serial_io, command_request_rx, command_response_tx, keying_event_tx);
            arduino_keyer_thread.thread_runner();
        });
        Self {
            command_request_tx: mutex_command_request_tx,
            command_response_rx,
            _thread_handle: thread_handle,
        }
    }

    fn transact_channels(&self, command: &str) -> Result<String, String> {
        match self.command_request_tx.lock().unwrap().send(command.to_string()) {
            Ok(_) => {
                match self.command_response_rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(result) => { match result {
                        Ok(response) => { Ok(response) }
                        Err(e) => { Err(e) }
                    } }
                    Err(timeout) => { Err(format!("Timeout: {}", timeout)) } // not ideal
                }
            }
            Err(send_error) => { Err(format!("SendError: {}", send_error)) }
        }
    }
}

impl Keyer for ArduinoKeyer {
    fn get_version(&mut self) -> Result<String, String> {
        let keyer_command = "v\n";
        self.transact_channels(keyer_command)
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
}

#[derive(Debug)]
pub enum KeyerState {
    Initial,
    KeyingDurationGetLSB, KeyingDurationGetMSB,
    ResponseGotGt, ResponseGotSpc, ResponseFinish
}

struct ArduinoKeyerThread {
    // Low-level serial access
    serial_io: Box<dyn SerialIO>,

    // Command channels
    command_request_rx: Receiver<String>,
    command_response_tx: Sender<Result<String, String>>,

    // Keying channel
    keying_event_tx: Sender<KeyingEvent>,

    // State machine data
    state: KeyerState,
    up: bool,
    duration: KeyerEdgeDurationMs,
    read_text: Vec<u8>,

}

impl ArduinoKeyerThread {
    fn new(serial_io: Box<dyn SerialIO>,
        command_request_rx: Receiver<String>,
           command_response_tx: Sender<Result<String, String>>,
        keying_event_tx: Sender<KeyingEvent>
    ) -> Self {
        debug!("Constructing ArduinoKeyerThread");
        Self {
            serial_io,
            command_request_rx,
            command_response_tx,
            keying_event_tx,
            state: Initial,
            up: false,
            duration: 0,
            read_text: vec![],
        }
    }

    // Thread that handles transactions asynchronously...
    // Requests/Responses cause the transact state machine to trigger, no support yet for
    // Notifications.
    fn thread_runner(&mut self) -> () {
        debug!("Keyer I/O thread started");
        // TODO until poisoned?
        loop {
            // Any incoming commands?
            match self.command_request_rx.try_recv() {
                Ok(command) => {
                    self.send_command(command.as_str());
                    // state machine will send to command_response_tx when done
                }
                Err(_) => {
                    // could timeout, or be disconnected?
                    // ignore for now...
                }
            }

            // Any keyer data?
            let mut read_buf: [u8; 1] = [0];
            let read_bytes = self.serial_io.read(&mut read_buf);
            match read_bytes {
                Ok(1) => {
                    debug!("state machine read {} state {:?} ", printable(read_buf[0]), self.state);
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
                    debug!("return from state routines: {:?}", next);
                }
                Ok(n) => {
                    warn!("In build loop, received {} bytes, but should be only 1?!", n);
                }
                Err(e) => {
                    warn!("Error in build loop: {}", e);
                    // With fake serial, there's no read timeout, so this is returned when the
                    // test data is exhausted and causes a busy loop.
                }
            }
        }
        // TODO when we swallow poison, exit here.
        //debug!("Keyer I/O thread stopped");
    }

    fn send_command(&mut self, command_to_keyer: &str) {
        debug!("Transact command [{}]", command_to_keyer);
        let written_bytes = self.serial_io.write(command_to_keyer.as_bytes());
        match written_bytes {
            Ok(n) => {
                debug!("Written {} bytes to keyer", n);
                self.set_state(Initial);
            }
            Err(e) => {
                warn!("Could not write command to keyer: {}", e.to_string())
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
            b'S' => {
                let event = Start();
                debug!("Keying: {}", event);
                self.keying_event_tx.send(event).unwrap();
            }
            b'E' => {
                let event = End();
                debug!("Keying: {}", event);
                self.keying_event_tx.send(event).unwrap();
            }
            b'+' => {
                self.up = false;
                self.duration = 0;
                self.set_state(KeyingDurationGetLSB);
            }
            b'-' => {
                self.up = true;
                self.duration = 0;
                self.set_state(KeyingDurationGetLSB);
            }
            _ => {
                warn!("Unexpected out-of-state data {}", printable(ch));
            }
        }
        None
    }

    fn keying_duration_get_lsb(&mut self, ch: u8) -> Option<Result<String, String>> {
        self.duration = ch as KeyerEdgeDurationMs;
        self.set_state(KeyingDurationGetMSB);
        None
    }

    fn keying_duration_get_msb(&mut self, ch: u8) -> Option<Result<String, String>> {
        self.duration |= (ch as KeyerEdgeDurationMs) << 8;
        let event = Timed(KeyingTimedEvent { up: self.up, duration: self.duration });
        debug!("Keying: {}", event);
        self.keying_event_tx.send(event).unwrap();
        self.set_state(Initial);
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
                let subslice = &self.read_text[0..self.read_text.len()];
                let string = String::from_utf8(Vec::from(subslice)).expect("Found invalid UTF-8");
                match self.command_response_tx.send(Ok(string)) {
                    Ok(_) => {}
                    Err(_) => {}
                }
                None
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
