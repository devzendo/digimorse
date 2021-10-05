use std::fmt::{Display, Formatter, Debug};
use std::fmt;
use actix::prelude::*;
use crate::libs::keyer_io::keyer_io::KeyingEvent::{Timed, Start, End};
use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Copy, Clone)]
pub enum KeyerMode {
    Straight, Paddle, // add Iambic A, Iambic B etc. later
}

#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Copy, Clone)]
pub enum KeyerPolarity {
    Normal, Reverse
}

#[derive(Serialize, Deserialize, Debug, PartialOrd, PartialEq, Copy, Clone)]
pub enum KeyerType {
    Arduino, Null
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
        let c = if self.up { '^' } else { 'v' };
        write!(f, "TIMED {} {}", c, self.duration)
    }
}

impl Debug for KeyingTimedEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = if self.up { '^' } else { 'v' };
        write!(f, "TIMED {} {}", c, self.duration)
    }
}

#[derive(Message)]
#[rtype(result = "()")]
#[derive(Clone, PartialEq)]
pub enum KeyingEvent {
    Timed(KeyingTimedEvent),
    Start(),
    End(),
}

pub struct KeyingEventReceiver {
    // This function is responsible for actually handling the KeyingEvent, so that different aspects
    // of the system can handle it in different ways, whilst maintaining a single Actor type that
    // handles KeyingEvents.
    // May need to enhance this fn to receive the Context
    pub dispatcher: Box<dyn FnMut(KeyingEvent) -> ()>,
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

    fn get_keyer_mode(&mut self) -> Result<KeyerMode, String>;
    fn set_keyer_mode(&mut self, mode: KeyerMode) -> Result<(), String>;

    fn get_keyer_polarity(&mut self) -> Result<KeyerPolarity, String>;
    fn set_keyer_polarity(&mut self, polarity: KeyerPolarity)-> Result<(), String>;
}

