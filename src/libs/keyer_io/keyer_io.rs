pub enum KeyingMode {
    Straight, Paddle, // add Iambic A, Iambic B etc. later
}

pub enum KeyerPolarity {
    Normal, Reverse
}

pub enum KeyerOutputMode {
    Edge, Timing
}

// Speed in WPM
pub type KeyerSpeed = u8;

// A keying edge (key down, key up) has just been detected. It's up to the receiver of this event
// to work out the duration of the up/down press/release - since the receipt of the last one.
#[readonly::make]
pub struct KeyingEdgeEvent {
    pub up: bool,
}

// A keying edge with duration.
#[readonly::make]
pub struct KeyingTimedEvent {
    pub up: bool, // The edge that has transitioned, key up or key down
    pub duration: KeyerEdgeDurationMs, // How long this edge lasted for
}
pub type KeyerEdgeDurationMs = u16; // 20ms is 60WPM dit, 720ms is 5WPM dah

// Listeners for edge or timed events.
pub trait KeyingEdgeEventListener {
    fn notify(&mut self, event: KeyingEdgeEvent);
}

pub trait KeyingTimedEventListener {
    fn notify(&mut self, event: KeyingTimedEvent);
}

pub trait Keyer {
    fn get_version(&self) -> Result<String, String>;

    fn get_speed(&self) -> Result<KeyerSpeed, String>;
    fn set_speed(&self, wpm: KeyerSpeed) -> Result<(), String>;

    fn get_keying_mode(&self) -> Result<KeyingMode, String>;
    fn set_keying_mode(&self, mode: KeyingMode)-> Result<(), String>;

    fn get_keyer_polarity(&self) -> Result<KeyerPolarity, String>;
    fn set_keyer_polarity(&self, polarity: KeyerPolarity)-> Result<(), String>;

    fn get_keyer_output_mode(&self) -> Result<KeyerOutputMode, String>;
    fn set_keyer_output_mode(&self, mode: KeyerOutputMode)-> Result<(), String>;

    fn set_edge_event_listener(&self, edge_event_listener: &mut dyn KeyingEdgeEventListener);
    fn clear_edge_event_listener(&self);
}


#[cfg(test)]
#[path = "./keyer_io_spec.rs"]
mod env_var_spec;
