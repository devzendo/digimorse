use crate::libs::keyer_io::keyer_io::KeyerSpeed;

// The rest of the system can effect changes in parts of the GUI via this facade...
pub trait GUIInput {

}

// The GUI controls can effect changes in the rest of the system via this facade...
pub trait GUIOutput {
    fn encode_and_send_text(&mut self, text: String);
    fn warning_beep(&mut self);
    fn set_keyer_speed(&mut self, keyer_speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;
}
