use crate::libs::keyer_io::keyer_io::{Keyer, KeyerSpeed, KeyingMode, KeyerPolarity, KeyingEvent};
use std::sync::mpsc::Sender;

pub struct NullKeyer {
    _keying_event_tx: Sender<KeyingEvent>,
    keyer_speed: KeyerSpeed,
    keying_mode: KeyingMode,
    keyer_polarity: KeyerPolarity,
}

impl NullKeyer {
    pub fn new(keying_event_tx: Sender<KeyingEvent>) -> Self {
        Self {
            _keying_event_tx: keying_event_tx,
            keyer_speed: KeyerSpeed::from(12),
            keying_mode: KeyingMode::Straight,
            keyer_polarity: KeyerPolarity::Normal,
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
        Ok(self.keying_mode)
    }

    fn set_keying_mode(&mut self, mode: KeyingMode) -> Result<(), String> {
        self.keying_mode = mode;
        Ok(())
    }

    fn get_keyer_polarity(&mut self) -> Result<KeyerPolarity, String> {
        Ok(self.keyer_polarity)
    }

    fn set_keyer_polarity(&mut self, polarity: KeyerPolarity) -> Result<(), String> {
        self.keyer_polarity = polarity;
        Ok(())
    }
}


#[cfg(test)]
#[path = "./null_keyer_io_spec.rs"]
mod null_keyer_io_spec;
