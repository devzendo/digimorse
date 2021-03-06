use std::fmt::{Display, Formatter, Debug};
use std::fmt;
use crate::libs::keyer_io::keyer_io::KeyingEvent::{Timed, Start, End};

pub enum KeyingMode {
    Straight, Paddle, // add Iambic A, Iambic B etc. later
}

pub enum KeyerPolarity {
    Normal, Reverse
}

// Speed in WPM
pub type KeyerSpeed = u8;

// A keying edge with duration.
#[derive(Clone, PartialEq)]
pub struct KeyingTimedEvent {
    pub up: bool, // The edge that has transitioned, key up or key down
    pub duration: KeyerEdgeDurationMs, // How long this edge lasted for
}

pub type KeyerEdgeDurationMs = u16; // 20ms is 60WPM dit, 720ms is 5WPM dah

impl Display for KeyingTimedEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = if self.up { '-' } else { '+' };
        write!(f, "TIMED {} {}", c, self.duration)
    }
}

impl Debug for KeyingTimedEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = if self.up { '-' } else { '+' };
        write!(f, "TIMED {} {}", c, self.duration)
    }
}

#[derive(Clone, PartialEq)]
pub enum KeyingEvent {
    Timed(KeyingTimedEvent),
    Start(),
    End(),
}

impl Display for KeyingEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Timed(timed) => {
                write!(f, "{}", timed)
            }
            Start() => {
                write!(f, "START")
            }
            End() => {
                write!(f, "END")
            }
        }
    }
}

impl Debug for KeyingEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Timed(timed) => {
                write!(f, "{}", timed)
            }
            Start() => {
                write!(f, "START")
            }
            End() => {
                write!(f, "END")
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
}

