extern crate hamcrest2;

#[cfg(test)]
mod source_encoder_spec {
    use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed, KeyingTimedEvent};
    use crate::libs::source_encoder::source_encoder::{DefaultSourceEncoder, SourceEncoder};
    use std::sync::mpsc::{Sender, Receiver};
    use std::sync::mpsc;
    use log::info;
    use std::env;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn default_keying_speed() {
        let (_keying_event_tx, keying_event_rx): (Sender<KeyingEvent>, Receiver<KeyingEvent>) = mpsc::channel();
        let source_encoder = DefaultSourceEncoder::new(keying_event_rx);

        assert_eq!(source_encoder.get_keyer_speed(), 12 as KeyerSpeed);
    }

    #[test]
    fn can_change_keying_speed() {
        let (_keying_event_tx, keying_event_rx): (Sender<KeyingEvent>, Receiver<KeyingEvent>) = mpsc::channel();
        let mut source_encoder = DefaultSourceEncoder::new(keying_event_rx);
        let new_keyer_speed: KeyerSpeed = 20;
        source_encoder.set_keyer_speed(new_keyer_speed);

        assert_eq!(source_encoder.get_keyer_speed(), new_keyer_speed);
    }


    #[test]
    fn encode_keying() {
        let (_keying_event_tx, keying_event_rx): (Sender<KeyingEvent>, Receiver<KeyingEvent>) = mpsc::channel();
        // define new encoding event, a type alias of vec u8?
        // create a encoding_tx, encoding_rx mpsc::channel and pass the encoding_tx to the encoder.
        // the loop below reads encodings and puts them in a vec for testing.
        // then inject some keyings
        let keyer_speed: KeyerSpeed = 20;
        let mut source_encoder = DefaultSourceEncoder::new(keying_event_rx);
        source_encoder.set_keyer_speed(keyer_speed);

        // inject these keyings...
        let _keyings = vec![
            KeyingEvent::Start(),

            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 30 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 30 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 10 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 10 }),

            KeyingEvent::End()
        ];
        // for x in keyings {
        //
        // }
        // info!("In wait loop for encodings...");
        // let mut received_keying_events: Vec<KeyingEvent> = vec!();
        // loop {
        //     let result = keying_event_rx.recv_timeout(Duration::from_millis(250));
        //     match result {
        //         Ok(keying_event) => {
        //             info!("Keying Event {}", keying_event);
        //             received_keying_events.push(keying_event);
        //         }
        //         Err(err) => {
        //             info!("timeout reading keying events channel {}", err);
        //             break
        //         }
        //     }
        // }
        info!("Out of keying wait loop");
    }
}