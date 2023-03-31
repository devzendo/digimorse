use crate::libs::keyer_io::keyer_io::KeyerSpeed;

// The rest of the system can effect changes in parts of the GUI via this facade...
pub trait GUIInput {
    fn set_rx_indicator(&mut self, state: bool);
    fn set_wait_indicator(&mut self, state: bool);
    fn set_tx_indicator(&mut self, state: bool);
    // TODO add downsampled FFT to waterfall
    // TODO add/clear dx station details for callsign/hash/offset
    // TODO add/clear receipt of costas array at offset
    // TODO clear decode frame
    // TODO add string to decode frame
}

// The GUI controls can effect changes in the rest of the system via this facade...
pub trait GUIOutput {
    fn encode_and_send_text(&mut self, text: String);
    fn warning_beep(&mut self);
    fn set_keyer_speed(&mut self, keyer_speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;
    // TODO filter disable (play everything)
    // TODO filter by range (left .. right) Hz
    // TODO filter by callsign/hash/offset
    // TODO set transmit offset
    // TODO enable/disable tuning output at current transmit offset
}

