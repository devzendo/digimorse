use std::io::ErrorKind;
use log::{debug, info, warn};

use crate::libs::keyer_io::arduino_keyer_io::KeyerState::{Initial, ResponseGotGt, ResponseGotSpc, ResponseFinish, KeyingDurationGetLSB, KeyingDurationGetMSB, WaitForEndOfComment};
use crate::libs::keyer_io::keyer_io::{Keyer, KeyerPolarity, KeyerMode, KeyingEvent, KeyerEdgeDurationMs, KeyingTimedEvent};
use crate::libs::serial_io::serial_io::SerialIO;
use crate::libs::util::util::printable;
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, mpsc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use bus::Bus;
use crate::libs::application::application::BusOutput;
use crate::libs::keyer_io::keyer_io::KeyingEvent::{Timed, Start, End};

pub struct ArduinoKeyer  {
    // Command channel to/from the thread. Sender is guarded by a Mutex to ensure a single command
    // in flight at a time.
    command_request_tx: Mutex<Sender<String>>,
    command_response_rx: Receiver<Result<String, String>>,
    // TODO only need a single channel for communicating commands and output-bus-setting between the
    // main thread and the keyer thread.
    keying_event_tx_request_tx: Sender<Arc<Mutex<Bus<KeyingEvent>>>>,

    terminate: Arc<AtomicBool>,
    thread_handle: Mutex<Option<JoinHandle<()>>>,
}

impl BusOutput<KeyingEvent> for ArduinoKeyer {
    fn clear_output_tx(&mut self) {
        todo!()
    }

    fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<KeyingEvent>>>) {
        match self.keying_event_tx_request_tx.send(output_tx) {
            Ok(_) => {
                // ok, no problem
            }
            Err(err) => {
                warn!("Could not send keying event bus to ArduinoKeyerThread: {}", err);
            }
        }
    }
}

impl ArduinoKeyer {
    pub fn new(serial_io: Box<dyn SerialIO>, terminate: Arc<AtomicBool>) -> Self {
        // Channels have two endpoints: the `Sender<T>` and the `Receiver<T>`,
        // where `T` is the type of the message to be transferred
        // (type annotation is superfluous)
        let (command_request_tx, command_request_rx): (Sender<String>, Receiver<String>) = mpsc::channel();
        let (command_response_tx, command_response_rx): (Sender<Result<String, String>>, Receiver<Result<String, String>>) = mpsc::channel();
        let (keying_event_tx_request_tx, keying_event_tx_request_rx): (Sender<Arc<Mutex<Bus<KeyingEvent>>>>, Receiver<Arc<Mutex<Bus<KeyingEvent>>>>) = mpsc::channel();
        let mutex_command_request_tx = Mutex::new(command_request_tx);

        let arc_terminate = terminate.clone();
        let thread_handle = thread::spawn(move || {
            let mut arduino_keyer_thread = ArduinoKeyerThread::new(serial_io, command_request_rx, command_response_tx, arc_terminate, keying_event_tx_request_rx);
            arduino_keyer_thread.thread_runner();
        });
        Self {
            command_request_tx: mutex_command_request_tx,
            command_response_rx,
            keying_event_tx_request_tx,
            terminate,
            thread_handle: Mutex::new(Some(thread_handle)),
        }
    }

    // Signals the thread to terminate, blocks on joining the handle. Used by drop().
    // Setting the terminate AtomicBool will allow the thread to stop on its own, but there's no
    // method other than this for blocking until it has actually stopped.
    pub fn terminate(&mut self) {
        debug!("Terminating keyer");
        self.terminate.store(true, Ordering::SeqCst);
        debug!("ArduinoKeyer joining thread handle...");
        let mut thread_handle = self.thread_handle.lock().unwrap();
        thread_handle.take().map(JoinHandle::join);
        debug!("ArduinoKeyer ...joined thread handle");
    }

    // Has the thread finished (ie has it been joined)?
    pub fn terminated(&mut self) -> bool {
        debug!("Is keyer terminated?");
        let ret = self.thread_handle.lock().unwrap().is_none();
        debug!("Termination state is {}", ret);
        ret
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


impl Drop for ArduinoKeyer {
    fn drop(&mut self) {
        debug!("ArduinoKeyer signalling termination to thread on drop");
        self.terminate();
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
        warn!("Setting the keyer speed is currently unimplemented");
        Ok(())
    }

    fn get_keyer_mode(&mut self) -> Result<KeyerMode, String> {
        unimplemented!()
    }

    fn set_keyer_mode(&mut self, _mode: KeyerMode) -> Result<(), String> {
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
    ResponseGotGt, ResponseGotSpc, ResponseFinish,
    WaitForEndOfComment
}

struct ArduinoKeyerThread {
    // Low-level serial access
    serial_io: Box<dyn SerialIO>,

    // Terminate flag
    terminate: Arc<AtomicBool>,

    // Command channels
    command_request_rx: Receiver<String>,
    command_response_tx: Sender<Result<String, String>>,
    keying_event_tx_request_rx: Receiver<Arc<Mutex<Bus<KeyingEvent>>>>,

    // Keying channel
    keying_event_tx: Option<Arc<Mutex<Bus<KeyingEvent>>>>,

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
        terminate: Arc<AtomicBool>,
        keying_event_tx_request_rx: Receiver<Arc<Mutex<Bus<KeyingEvent>>>>
    ) -> Self {
        debug!("Constructing ArduinoKeyerThread");
        Self {
            serial_io,
            terminate,
            command_request_rx,
            command_response_tx,
            keying_event_tx_request_rx,
            keying_event_tx: None,
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
        info!("Keyer I/O thread started");
        loop {
            if self.terminate.load(Ordering::SeqCst) {
                info!("Terminating keyer I/O thread");
                break;
            }
            match self.keying_event_tx_request_rx.try_recv() {
                Ok(bus) => {
                    self.keying_event_tx = Some(bus);
                }
                Err(_) => {
                    // ignore for now
                }
            }
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
                        KeyerState::WaitForEndOfComment => {
                            self.wait_for_end_of_comment(read_buf[0])
                        }

                    };
                    debug!("return from state routines: {:?}", next);
                }
                Ok(n) => {
                    warn!("In build loop, received {} bytes, but should be only 1?!", n);
                }
                Err(e) => {
                    match e.kind() {
                        // With fake serial, there's no read timeout, so this is returned when the
                        // test data is exhausted and causes a busy loop.
                        ErrorKind::UnexpectedEof => {
                            warn!("End of build loop: {}", e);
                            break;
                        }
                        _ => {
                            // Be silent when there's nothing incoming..
                        }
                    }
                }
            }
        }
        // TODO when we swallow poison, exit here.
        info!("Keyer I/O thread stopped");
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

    fn send(&mut self, event: KeyingEvent) {
        if self.keying_event_tx.is_none() {
            warn!("Cannot send event {} as there is no current output bus", event);
            return;
        }
        let ref_opt_bus = &mut self.keying_event_tx;
        ref_opt_bus.as_mut().map( |bus| bus.lock().unwrap().broadcast(event));
    }

    fn initial(&mut self, ch: u8) -> Option<Result<String, String>> {
        match ch {
            b'#' => {
                self.read_text.clear();
                self.set_state(WaitForEndOfComment);
            }
            b'>' => {
                self.read_text.clear();
                self.set_state(ResponseGotGt);
            }
            b'S' => {
                let event = Start();
                debug!("Keying: {}", event);
                self.send(event);
            }
            b'E' => {
                let event = End();
                debug!("Keying: {}", event);
                self.send(event);
            }
            b'+' => {
                self.up = false;
                self.duration = 0;
                self.set_state(KeyingDurationGetMSB);
            }
            b'-' => {
                self.up = true;
                self.duration = 0;
                self.set_state(KeyingDurationGetMSB);
            }
            // For tests, to get other threads active without this spinning, just delay a bit..
            b'_' => {
                thread::sleep(Duration::from_millis(2));
            }
            _ => {
                warn!("Unexpected out-of-state data {}", printable(ch));
            }
        }
        None
    }

    fn keying_duration_get_lsb(&mut self, ch: u8) -> Option<Result<String, String>> {
        self.duration |= (ch as KeyerEdgeDurationMs) & 0x00FF;
        let event = Timed(KeyingTimedEvent { up: self.up, duration: self.duration });
        debug!("Keying: {}", event);
        self.send(event);
        self.set_state(Initial);
        None
    }

    fn keying_duration_get_msb(&mut self, ch: u8) -> Option<Result<String, String>> {
        self.duration = ((ch as KeyerEdgeDurationMs) << 8) & 0xFF00;
        self.set_state(KeyingDurationGetLSB);
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

    fn wait_for_end_of_comment(&mut self, ch: u8) -> Option<Result<String, String>> {
        return match ch {
            b'\n' => {
                self.set_state(Initial);
                None
            }
            _ => {
                debug!("Ignoring comment data {}", printable(ch));
                None
            }
        }
    }
}


#[cfg(test)]
#[path = "./arduino_keyer_io_spec.rs"]
mod arduino_keyer_io_spec;
