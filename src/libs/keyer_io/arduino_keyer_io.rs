use crate::libs::keyer_io::keyer_io::{Keyer, KeyingEdgeEventListener, KeyerPolarity, KeyingMode, KeyerOutputMode};
use crate::libs::serial_io::serial_io::SerialIO;

pub struct ArduinoKeyer {
    serial_io: dyn SerialIO,
    // thread waiting for data, or async read/promise
    // current callback to receive non-pulse/timing data (lines starting with > until blank NL)
}

impl ArduinoKeyer {
/*    fn transact(&self, command: String) -> Result<String, String> {
        // set callback to build up
        // send(command);
    }
*/
}
impl Keyer for ArduinoKeyer {
    fn get_version(&self) -> Result<String, String> {
        unimplemented!()
    }

    fn get_speed(&self) -> Result<u8, String> {
        unimplemented!()
    }

    fn set_speed(&self, _wpm: u8) -> Result<(), String> {
        unimplemented!()
    }

    fn get_keying_mode(&self) -> Result<KeyingMode, String> {
        unimplemented!()
    }

    fn set_keying_mode(&self, _mode: KeyingMode) -> Result<(), String> {
        unimplemented!()
    }

    fn get_keyer_polarity(&self) -> Result<KeyerPolarity, String> {
        unimplemented!()
    }

    fn set_keyer_polarity(&self, _polarity: KeyerPolarity) -> Result<(), String> {
        unimplemented!()
    }

    fn get_keyer_output_mode(&self) -> Result<KeyerOutputMode, String> {
        unimplemented!()
    }

    fn set_keyer_output_mode(&self, _mode: KeyerOutputMode) -> Result<(), String> {
        unimplemented!()
    }

    fn set_edge_event_listener(&self, _pulse_event_listener: &mut dyn KeyingEdgeEventListener) {
        unimplemented!()
    }

    fn clear_edge_event_listener(&self) {
        unimplemented!()
    }
}