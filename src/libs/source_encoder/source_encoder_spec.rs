extern crate hamcrest2;

#[cfg(test)]
mod source_encoder_spec {
    use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed, KeyingTimedEvent};
    use crate::libs::source_encoder::source_encoder::{DefaultSourceEncoder, SourceEncoder};
    use log::info;
    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use bus::Bus;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[test]
    fn default_keying_speed() {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        let mut source_encooder_tx = Bus::new(16);
        // let source_encoder_rx = source_encooder_tx.add_rx();
        let source_encoder = DefaultSourceEncoder::new(keying_event_rx, source_encooder_tx, terminate.clone());

        assert_eq!(source_encoder.get_keyer_speed(), 12 as KeyerSpeed);
    }

    #[test]
    fn can_change_keying_speed() {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        let mut source_encooder_tx = Bus::new(16);
        // let source_encoder_rx = source_encooder_tx.add_rx();
        let mut source_encoder = DefaultSourceEncoder::new(keying_event_rx, source_encooder_tx, terminate.clone());
        let new_keyer_speed: KeyerSpeed = 20;
        source_encoder.set_keyer_speed(new_keyer_speed);

        assert_eq!(source_encoder.get_keyer_speed(), new_keyer_speed);
    }


    #[test]
    fn encode_keying() {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        // define new encoding event, a type alias of vec u8?
        // create a encoding_tx, encoding_rx mpsc::channel and pass the encoding_tx to the encoder.
        // the loop below reads encodings and puts them in a vec for testing.
        // then inject some keyings
        let keyer_speed: KeyerSpeed = 20;
        let mut source_encooder_tx = Bus::new(16);
        // let source_encoder_rx = source_encooder_tx.add_rx();
        let mut source_encoder = DefaultSourceEncoder::new(keying_event_rx, source_encooder_tx, terminate.clone());
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
