use std::sync::{Arc, Mutex};
use crate::libs::keyer_io::keyer_io::{Keyer, KeyerSpeed, KeyerMode, KeyerPolarity, KeyingEvent};
use std::sync::mpsc::Sender;
use bus::Bus;
use crate::libs::application::application::BusOutput;

pub struct NullKeyer {
    _keying_event_tx: Sender<KeyingEvent>,
    keyer_speed: KeyerSpeed,
    keyer_mode: KeyerMode,
    keyer_polarity: KeyerPolarity,
}

impl NullKeyer {
    pub fn new(keying_event_tx: Sender<KeyingEvent>) -> Self {
        Self {
            _keying_event_tx: keying_event_tx,
            keyer_speed: KeyerSpeed::from(12),
            keyer_mode: KeyerMode::Straight,
            keyer_polarity: KeyerPolarity::Normal,
        }
    }
}

impl BusOutput<KeyingEvent> for NullKeyer {
    fn clear_output_tx(&mut self) {
        // does nothing since this does not output KeyingEvents
    }

    fn set_output_tx(&mut self, _output_tx: Arc<Mutex<Bus<KeyingEvent>>>) {
        // does nothing since this does not output KeyingEvents
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

    fn get_keyer_mode(&mut self) -> Result<KeyerMode, String> {
        Ok(self.keyer_mode)
    }

    fn set_keyer_mode(&mut self, mode: KeyerMode) -> Result<(), String> {
        self.keyer_mode = mode;
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
