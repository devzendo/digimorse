extern crate hamcrest2;

#[cfg(test)]
mod null_keyer_io_spec {
    use crate::libs::keyer_io::null_keyer_io::NullKeyer;
    use crate::libs::keyer_io::keyer_io::{Keyer, KeyerSpeed, KeyingEvent, KeyerMode, KeyerPolarity};
    use std::sync::mpsc::{Sender, Receiver};
    use std::sync::mpsc;
    use std::env;
    use hamcrest2::prelude::*;

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
        assert_that!(keyer.set_speed(KeyerSpeed::from(20)), has(()) );
        assert_eq!(keyer.get_speed(), Ok(KeyerSpeed::from(20)));
    }

    #[test]
    fn get_set_mode() {
        let mut keyer = keyer();
        assert_eq!(keyer.get_keyer_mode(), Ok(KeyerMode::Straight));
        assert_that!(keyer.set_keyer_mode(KeyerMode::Paddle), has(()) );
        assert_eq!(keyer.get_keyer_mode(), Ok(KeyerMode::Paddle));
    }

    #[test]
    fn get_set_polarity() {
        let mut keyer = keyer();
        assert_eq!(keyer.get_keyer_polarity(), Ok(KeyerPolarity::Normal));
        assert_that!(keyer.set_keyer_polarity(KeyerPolarity::Reverse), has(()) );
        assert_eq!(keyer.get_keyer_polarity(), Ok(KeyerPolarity::Reverse));
    }
}
