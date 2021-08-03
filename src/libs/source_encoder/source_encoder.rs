use std::sync::mpsc::{Sender, Receiver};
use std::sync::{mpsc, Mutex};

use log::{info, warn};

use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed};

pub trait SourceEncoder {
    fn set_keyer_speed(&mut self, speed: KeyerSpeed);
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