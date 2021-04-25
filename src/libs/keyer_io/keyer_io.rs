use std::fmt::{Display, Formatter};
use std::fmt;
use crate::libs::keyer_io::keyer_io::KeyingEvent::{Edge, Timed};

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
#[derive(Clone)]
pub struct KeyingEdgeEvent {
    pub up: bool,
}
impl Display for KeyingEdgeEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = if self.up { '+' } else { '-' };
        write!(f, "EDGE {}", c)
    }
}

// A keying edge with duration.
#[derive(Clone)]
pub struct KeyingTimedEvent {
    pub up: bool, // The edge that has transitioned, key up or key down
    pub duration: KeyerEdgeDurationMs, // How long this edge lasted for
}
pub type KeyerEdgeDurationMs = u16; // 20ms is 60WPM dit, 720ms is 5WPM dah
impl Display for KeyingTimedEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = if self.up { '+' } else { '-' };
        write!(f, "TIMED {} {}", c, self.duration)
    }
}

#[derive(Clone)]
pub enum KeyingEvent {
    Edge(KeyingEdgeEvent),
    Timed(KeyingTimedEvent),
}

impl Display for KeyingEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Edge(edge) => {
                write!(f, "{}", edge)
            }
            Timed(timed) => {
                write!(f, "{}", timed)
            }
        }
    }
}

pub trait Keyer {
    fn get_version(&mut self) -> Result<String, String>;

    fn get_speed(&mut self) -> Result<KeyerSpeed, String>;
    fn set_speed(&mut self, wpm: KeyerSpeed) -> Result<(), String>;

    fn get_keying_mode(&mut self) -> Result<KeyingMode, String>;
    fn set_keying_mode(&mut self, mode: KeyingMode)-> Result<(), String>;

    fn get_keyer_polarity(&mut self) -> Result<KeyerPolarity, String>;
    fn set_keyer_polarity(&mut self, polarity: KeyerPolarity)-> Result<(), String>;

    fn get_keyer_output_mode(&mut self) -> Result<KeyerOutputMode, String>;
    fn set_keyer_output_mode(&mut self, mode: KeyerOutputMode)-> Result<(), String>;
}

