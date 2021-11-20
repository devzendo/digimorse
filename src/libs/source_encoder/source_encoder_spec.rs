extern crate hamcrest2;

#[cfg(test)]
mod source_encoder_spec {
    use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed, KeyingTimedEvent};
    use crate::libs::source_encoder::source_encoder::{DefaultSourceEncoder, SourceEncoder, SourceEncoding};
    use bus::{Bus, BusReader};
    use log::info;
    use pretty_hex::*;
    use rstest::*;
    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct SourceEncoderFixture {
        terminate: Arc<AtomicBool>,
        keying_event_tx: Bus<KeyingEvent>,
        source_encoder_rx: BusReader<SourceEncoding>,
        source_encoder: DefaultSourceEncoder,
    }

    #[fixture]
    fn fixture() -> SourceEncoderFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        let mut source_encoder_tx = Bus::new(16);
        let source_encoder_rx = source_encoder_tx.add_rx();
        let source_encoder = DefaultSourceEncoder::new(keying_event_rx, source_encoder_tx, terminate.clone());
        SourceEncoderFixture {
            terminate,
            keying_event_tx,
            source_encoder_rx,
            source_encoder
        }
    }

    #[rstest]
    pub fn default_keying_speed(fixture: SourceEncoderFixture) {
        assert_eq!(fixture.source_encoder.get_keyer_speed(), 12 as KeyerSpeed);
    }

    #[rstest]
    fn can_change_keying_speed(mut fixture: SourceEncoderFixture) {
        let new_keyer_speed: KeyerSpeed = 20;
        fixture.source_encoder.set_keyer_speed(new_keyer_speed);

        assert_eq!(fixture.source_encoder.get_keyer_speed(), new_keyer_speed);
    }

    #[rstest]
    fn emit_with_no_keying_data_emits_nothing(_fixture: SourceEncoderFixture) {}

    #[rstest]
    fn emit_with_just_start_keying_data_emits_nothing(_fixture: SourceEncoderFixture) {}

    #[rstest]
    fn emit_with_some_keying_data_emits_with_padding(_fixture: SourceEncoderFixture) {}

    #[rstest]
    fn emit_with_some_keying_data_emits_with_padding_then_next_emit_emits_nothing(_fixture:
                                                                                  SourceEncoderFixture) {}


    //#[rstest]
    fn encode_keying(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move|| {

            // define new encoding event, a type alias of vec u8?
            // the loop below reads encodings and puts them in a vec for testing.
            // then inject some keyings
            let keyer_speed: KeyerSpeed = 20;
            fixture.source_encoder.set_keyer_speed(keyer_speed);

            // inject these keyings...
            let keyings = vec![
                KeyingEvent::Start(),
                KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 210 }), // C
                KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 72 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 73 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 64 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 250 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 65 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 61 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 298 }), // inter-letter gap
                KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 234 }),  // Q
                KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 45 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 208 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 77 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 78 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 56 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 323 }),
                KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 596 }), // inter-word gap
                KeyingEvent::End()
            ];
            for k in keyings {
                fixture.keying_event_tx.broadcast(k);
            }
            // Force the encoder to emit a frame
            fixture.source_encoder.emit();
            let result = fixture.source_encoder_rx.recv();
            match result {
                Ok(source_encoding) => {
                    info!("encode_keying: isEnd {}", source_encoding.is_end);
                    let hexdump = pretty_hex(&source_encoding.block);
                    let hexdump_lines = hexdump.split("\n");
                    for line in hexdump_lines {
                        info!("encode_keying: Encoding {}", line);
                    }
                }
                Err(err) => {
                    panic!("encode_keying: error reading encoder bus {}", err);
                }
            }
        })
    }
}
