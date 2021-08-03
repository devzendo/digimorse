use std::sync::mpsc::{Sender, Receiver};
use std::sync::{mpsc, Mutex};

use log::{info, warn};

use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed};

pub trait SourceEncoder {
    fn set_keyer_speed(&mut self, speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;
}

#[readonly::make]
pub struct DefaultSourceEncoder {
    keyer_speed: KeyerSpeed,
    keying_event_rx: Receiver<KeyingEvent>,

}

impl DefaultSourceEncoder {
    pub fn new(keying_event_rx: Receiver<KeyingEvent>) -> Self {
        Self {
            keyer_speed: 12,
            keying_event_rx: keying_event_rx
        }
    }
}

impl SourceEncoder for DefaultSourceEncoder {
    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keyer_speed = speed;
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }
}

#[cfg(test)]
#[path = "./source_encoder_spec.rs"]
mod source_encoder_spec;
