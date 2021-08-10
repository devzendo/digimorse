extern crate hamcrest2;

use log::{debug, warn};
use crate::libs::serial_io::serial_io::SerialIO;
use crate::libs::util::util::*;
use std::io;
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{Sender};
use std::time::Duration;

#[cfg(test)]
mod null_keyer_io_spec {
    use crate::libs::keyer_io::null_keyer_io::NullKeyer;
    use crate::libs::keyer_io::keyer_io::{Keyer, KeyerSpeed, KeyingEvent, KeyingTimedEvent, KeyingMode, KeyerPolarity};
    use std::sync::mpsc::{Sender, Receiver};
    use std::sync::mpsc;
    use log::info;
    use std::time::Duration;
    use std::env;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    fn keyer() -> NullKeyer {
        let (keying_event_tx, _keying_event_rx): (Sender<KeyingEvent>, Receiver<KeyingEvent>) = mpsc::channel();
        NullKeyer::new(keying_event_tx)
    }

    #[test]
    fn get_version() {
        let mut keyer = keyer();
        match keyer.get_version() {
            Ok(v) => {
                // Keyer replied with....
                assert_eq!(v, "v1.0.0");
            }
            Err(e) => {
                panic!("Did not get version: {}", e);
            }
        }
    }

    #[test]
    fn get_set_speed() {
        let mut keyer = keyer();
        assert_eq!(keyer.get_speed(), Ok(KeyerSpeed::from(12)));
        keyer.set_speed(KeyerSpeed::from(20));
        assert_eq!(keyer.get_speed(), Ok(KeyerSpeed::from(20)));
    }

    #[test]
    fn get_set_mode() {
        let mut keyer = keyer();
        assert_eq!(keyer.get_keying_mode(), Ok(KeyingMode::Straight));
        keyer.set_keying_mode(KeyingMode::Paddle);
        assert_eq!(keyer.get_keying_mode(), Ok(KeyingMode::Paddle));
    }

    #[test]
    fn get_set_polarity() {
        let mut keyer = keyer();
        assert_eq!(keyer.get_keyer_polarity(), Ok(KeyerPolarity::Normal));
        keyer.set_keyer_polarity(KeyerPolarity::Reverse);
        assert_eq!(keyer.get_keyer_polarity(), Ok(KeyerPolarity::Reverse));
    }
}
