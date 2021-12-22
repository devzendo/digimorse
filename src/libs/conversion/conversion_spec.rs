extern crate hamcrest2;

#[cfg(test)]
mod conversion_spec {
    use hamcrest2::prelude::*;
    use crate::libs::conversion::conversion::text_to_keying;
    use crate::libs::conversion::paris::PARIS_KEYING_12WPM;
    use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyingTimedEvent};

    #[test]
    pub fn test_text_to_keying_no_space() {
        let actual_keying = text_to_keying(12, "PARIS");
        assert_that!(actual_keying, equal_to(PARIS_KEYING_12WPM.clone()));
    }

    #[test]
    pub fn test_text_to_keying_with_space() {
        let expected_keying = vec![
            KeyingEvent::Start(),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 700 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 100 }),
            KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 300 }),
            KeyingEvent::End(),
        ];

        let actual_keying = text_to_keying(12, "C Q");
        assert_that!(actual_keying, equal_to(expected_keying));
    }
}
