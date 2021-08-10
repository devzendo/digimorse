use log::{warn, debug};

use crate::libs::keyer_io::keyer_io::{Keyer, KeyerSpeed, KeyingMode, KeyerPolarity, KeyingEvent};
use crate::libs::util::util::printable;
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{mpsc, Mutex};
use std::time::Duration;
use crate::libs::keyer_io::keyer_io::KeyingEvent::{Timed, Start, End};

pub struct NullKeyer {
    keying_event_tx: Sender<KeyingEvent>,
    keyer_speed: KeyerSpeed,
}

impl NullKeyer {
    pub fn new(keying_event_tx: Sender<KeyingEvent>) -> Self {
        Self {
            keying_event_tx: keying_event_tx,
            keyer_speed: KeyerSpeed::from(12),

        }
    }
}

impl Keyer for NullKeyer {
    fn get_version(&mut self) -> Result<String, String> {
        Ok("v1.0.0".to_owned())
    }

    fn get_speed(&mut self) -> Result<KeyerSpeed, String> {
        Ok(self.keyer_speed)
    }

    fn set_speed(&mut self, wpm: KeyerSpeed) -> Result<(), String> {
        self.keyer_speed = wpm;
        Ok(())
    }

    fn get_keying_mode(&mut self) -> Result<KeyingMode, String> {
        todo!()
    }

    fn set_keying_mode(&mut self, mode: KeyingMode) -> Result<(), String> {
        todo!()
    }

    fn get_keyer_polarity(&mut self) -> Result<KeyerPolarity, String> {
        todo!()
    }

    fn set_keyer_polarity(&mut self, polarity: KeyerPolarity) -> Result<(), String> {
        todo!()
    }
}


#[cfg(test)]
#[path = "./null_keyer_io_spec.rs"]
mod null_keyer_io_spec;
