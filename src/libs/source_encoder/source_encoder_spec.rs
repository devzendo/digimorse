extern crate hamcrest2;

#[cfg(test)]
mod source_encoder_spec {
    use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed, KeyingTimedEvent};
    use crate::libs::source_encoder::source_encoder::{DefaultSourceEncoder, SourceEncoder, SourceEncoding};
    use crate::libs::source_encoder::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};
    use bus::{Bus, BusReader};
    use log::{debug, info};
    use hamcrest2::prelude::*;
    use pretty_hex::*;
    use rstest::*;
    use std::{env, thread};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    fn wait_5_ms() {
        thread::sleep(Duration::from_millis(5));
    }

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

        info!("Fixture setup sleeping");
        wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        SourceEncoderFixture {
            terminate,
            keying_event_tx,
            source_encoder_rx,
            source_encoder
        }
    }

    impl Drop for SourceEncoderFixture {
        fn drop(&mut self) {
            debug!("SourceEncoderFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            wait_5_ms();
            debug!("SourceEncoderFixture ...set terminate flag");
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
    fn emit_after_no_keying_data_emits_nothing(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.emit();
            wait_5_ms();

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(e) => {
                    panic!("Should not have received a SourceEncoding of {}", e);
                }
                Err(_) => {
                    info!("Correctly timed out");
                }
            }
        });
    }

    #[rstest]
    fn emit_after_just_start_keying_data_emits_nothing(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            wait_5_ms();
            fixture.source_encoder.emit();
            wait_5_ms();

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(e) => {
                    panic!("Should not have received a SourceEncoding of {}", e);
                }
                Err(_) => {
                    info!("Correctly timed out");
                }
            }
        });
    }

    #[rstest]
    fn emit_after_some_keying_data_emits_single_polarity_wpm_and_perfect_dits_with_padding(mut
                                                                                         fixture:
                                                                 SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            let keyer_speed: KeyerSpeed = 20;
            fixture.source_encoder.set_keyer_speed(keyer_speed);
            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            // A precise dit at 20WPM is 60ms long.
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true,
                duration: 60 }));
            // inter-element dit
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false,
                duration: 60 }));
            // another dit
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true,
                duration: 60 }));

            wait_5_ms();
            fixture.source_encoder.emit();
            wait_5_ms();

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
                    //                                    F:PD        F:PD
                    //                     F:WPWPM-    --P    F:    PD
                    assert_eq!(vec, vec![0b00010101, 0b00101100, 0b11001100, 0, 0, 0, 0, 0]);
                    // Got                 1   5       2    C       C   C     00 00 00 00 00
                    assert_eq!(encoding.is_end, false);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }

    #[rstest]
    fn keyer_speed_is_passed_to_the_keying_encoder(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            // A precise dit at 20WPM is 60ms long.
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            // Change speed, send another perfect dit at that speed - should get two perfect dits
            // encoded
            fixture.source_encoder.set_keyer_speed(40);
            // inter-element dit
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 30 }));

            wait_5_ms();
            fixture.source_encoder.emit();
            wait_5_ms();

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));

                    // TODO should get a second WPM/Polarity frame if the speed changes in the
                    // middle of a block?
                    //                                    F:PD
                    //                     F:WPWPM-    --P    F:    PD
                    assert_eq!(vec, vec![0b00010101, 0b00101100, 0b11000000, 0, 0, 0, 0, 0]);
                    assert_eq!(encoding.is_end, false);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }

    #[rstest]
    fn emit_with_some_keying_data_emits_with_padding_then_next_emit_emits_nothing(_fixture:
                                                                                  SourceEncoderFixture) {}


    // TODO keying with end, emit, sets the end flag in the SourceEncoding

    // TODO keying with end, emit, then more keying and emit has a cleared end flag in
    // the second SourceEncoding

    // TODO flag indicating wpm|polarity sent gets reset on each new frame's first keying

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
