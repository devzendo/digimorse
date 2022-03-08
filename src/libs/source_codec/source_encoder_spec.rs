extern crate hamcrest2;

#[cfg(test)]
mod source_encoder_spec {
    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use bus::{Bus, BusReader};
    use hamcrest2::prelude::*;
    use log::{debug, info};
    use rstest::*;

    use crate::libs::keyer_io::keyer_io::{KeyerSpeed, KeyingEvent, KeyingTimedEvent};
    use crate::libs::source_codec::source_encoder::{SourceEncoder, SourceEncoding};
    use crate::libs::source_codec::source_encoding::Frame;
    use crate::libs::source_codec::test_encoding_builder::encoded;
    use crate::libs::util::test_util;

    const TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS: usize = 64;

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
        source_encoder: SourceEncoder,
    }

    #[fixture]
    fn fixture() -> SourceEncoderFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        let mut source_encoder_tx = Bus::new(16);
        let source_encoder_rx = source_encoder_tx.add_rx();
        let source_encoder = SourceEncoder::new(keying_event_rx, source_encoder_tx, terminate.clone(), TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);

        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
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
            test_util::wait_5_ms();
            debug!("SourceEncoderFixture ...set terminate flag");
        }
    }


    #[test]
    #[should_panic]
    fn block_size_must_be_a_multiple_of_8_not_0() {
        try_block_size(0);
    }

    #[test]
    #[should_panic]
    fn block_size_must_be_a_multiple_of_8_not_7() {
        try_block_size(7);
    }

    #[test]
    #[should_panic]
    fn block_size_must_be_a_multiple_of_8_not_9() {
        try_block_size(9);
    }

    #[test]
    fn block_size_must_be_a_multiple_of_8() {
        try_block_size(8);
    }

    fn try_block_size(block_size: usize) {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut keying_event_tx = Bus::new(16);
        let keying_event_rx = keying_event_tx.add_rx();
        let mut source_encoder_tx = Bus::new(16);
        let source_encoder_rx = source_encoder_tx.add_rx();
        let _ = SourceEncoder::new(keying_event_rx, source_encoder_tx, terminate.clone(), block_size);
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
            test_util::wait_5_ms();

            should_timeout(fixture)
        });
    }

    #[rstest]
    fn emit_after_just_start_keying_data_emits_nothing(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            should_timeout(fixture)
        });
    }

    fn should_timeout(mut fixture: SourceEncoderFixture) {
        match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(e) => {
                panic!("Should not have received a SourceEncoding of {}", e);
            }
            Err(_) => {
                info!("Correctly timed out");
            }
        }
    }

    fn expect_block_with_expected_end(fixture: &mut SourceEncoderFixture, expected_end: bool) {
        match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(encoding) => {
                info!("Received SourceEncoding of {}", encoding);
                assert_eq!(encoding.is_end, expected_end);
            }
            Err(e) => {
                panic!("Should have received a SourceEncoding, not an error of {}", e);
            }
        }
    }


    #[rstest]
    fn first_keying_after_start_generates_wpm_and_mark_polarity_then_keying(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            start_single_dit_emit(&mut fixture);

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;

                    //                                    F:PD
                    //                     F:WPWPM-    --P
                    assert_eq!(vec, vec![0b00010101, 0b00101100, 0, 0, 0, 0, 0, 0]);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }


    #[rstest]
    fn keying_does_not_set_the_end_flag(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            start_single_dit_emit(&mut fixture);

            expect_block_with_expected_end(&mut fixture, false);
        });
    }

    #[rstest]
    fn block_vec_is_the_right_size(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            start_single_dit_emit(&mut fixture);

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    assert_that!(&vec, len(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }

    fn start_single_dit_emit(fixture: &mut SourceEncoderFixture) {
        fixture.source_encoder.set_keyer_speed(20);
        test_util::wait_5_ms();

        fixture.keying_event_tx.broadcast(KeyingEvent::Start());
        // A precise dit at 20WPM is 60ms long.
        fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
        test_util::wait_5_ms();

        fixture.source_encoder.emit();
        test_util::wait_5_ms();
    }

    #[rstest]
    fn emit_after_some_keying_data_emits_single_polarity_wpm_and_perfect_dits_with_padding(mut fixture: SourceEncoderFixture) {
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

            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    assert_that!(&vec, len(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
                    //                                    F:PD        F:PD
                    //                     F:WPWPM-    --P    F:    PD
                    assert_eq!(vec, vec![0b00010101, 0b00101100, 0b11001100, 0, 0, 0, 0, 0]);
                    // Got                 1   5       2    C       C   C     00 00 00 00 00
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }

    #[rstest]
    fn keyer_speed_is_passed_to_the_keying_encoder_and_causes_another_wpmpolarity_to_be_emitted(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            // A precise dit at 20WPM is 60ms long.
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            test_util::wait_5_ms();

            // Should see a WPM/Polarity and a perfect dit. Change speed, send another perfect dit
            // at that speed - should get another WPN/Polarity and a second perfect dit.
            fixture.source_encoder.set_keyer_speed(40);
            // inter-element dit
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 30 }));

            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    //                                    F:PD                   F:PD
                    //                     F:WPWPM-    --P    F    :WPWPM--    -P
                    assert_eq!(vec, vec![0b00010101, 0b00101100, 0b00110100, 0b00011000, 0, 0, 0, 0]);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }

    #[rstest]
    fn
    keyer_speed_changes_near_end_of_block_and_wpmpolarity_wont_fit_so_first_block_is_emitted_and_wpmpolarity_and_keying_is_in_second_block(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            test_util::wait_5_ms();

            // First frame should see a WPM/Polarity and some perfect dits, then padding.
            //
            // Change speed, send another perfect dit at that speed - the second frame should see a
            // WPN/Polarity and the final perfect dit.

            fixture.source_encoder.set_keyer_speed(40);
            // inter-element dit
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 30 }));

            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            // Block 1
            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    //                                    F:PD        F:PD        F:PD
                    //                     F:WPWPM-    --P    F    :PD    F    :PD    F
                    assert_eq!(vec, vec![0b00010101, 0b00101100, 0b11001100, 0b11001100,
                    //                        F:PD        F:PD        F:PD        F:WP1234567
                    //                     :PD    F    :PD    F    :PD    F    :PD
                                         0b11001100, 0b11001100, 0b11001100, 0b11000000]);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
            // Block 2
            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    //                                    F:PD
                    //                     F:WPWPM-    --P
                    assert_eq!(vec, vec![0b00011010, 0b00101100, 0, 0, 0, 0, 0, 0]);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }

    #[rstest]
    fn keying_wont_fit_in_first_block_so_goes_in_next_block_after_wpmpolarity(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            // This one won't fit in the block..
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));

            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            // Block 1
            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    //                                    F:PD        F:PD        F:PD
                    //                     F:WPWPM-    --P    F    :PD    F    :PD    F
                    assert_eq!(vec, vec![0b00010101, 0b00101100, 0b11001100, 0b11001100,
                    //                        F:PD        F:PD        F:PD        F:PD
                    //                     :PD    F    :PD    F    :PD    F    :PD
                                         0b11001100, 0b11001100, 0b11001100, 0b11001100]);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
            // Block 2
            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    //                                    F:PD
                    //                     F:WPWPM-    --P
                    assert_eq!(vec, vec![0b00010101, 0b00001100, 0, 0, 0, 0, 0, 0]);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }

    #[rstest]
    fn emit_with_some_keying_data_emits_with_padding_then_next_emit_emits_nothing(mut fixture:
                                                                                  SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            // Block 1
            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    //                                    F:PD
                    //                     F:WPWPM-    --P
                    assert_eq!(vec, vec![0b00010101, 0b00101100, 0, 0, 0, 0, 0, 0]);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
            // No Block 2
            should_timeout(fixture);
        });
    }

    #[rstest]
    fn keying_with_end_sets_the_end_flag(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::End());
            test_util::wait_5_ms();

            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            expect_block_with_expected_end(&mut fixture, true);
        });
    }

    #[rstest]
    fn the_end_flag_is_cleared_after_emitting(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::End());
            test_util::wait_5_ms();

            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            test_util::wait_5_ms();

            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            // Block 1
            expect_block_with_expected_end(&mut fixture, true);
            // Block 2
            expect_block_with_expected_end(&mut fixture, false);
        });
    }

    #[rstest]
    fn keying_with_end_emits_an_end_frame(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::End());
            test_util::wait_5_ms();

            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;

                    let expected_encoding = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
                        Frame::KeyingEnd,
                    ]);
                    assert_eq!(vec, expected_encoding);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }

    #[rstest]
    fn end_wont_fit_in_first_block_so_goes_in_next_block(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            // Won't fit.
            fixture.keying_event_tx.broadcast(KeyingEvent::End());

            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            // Block 1
            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    let expected_encoding = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
                        Frame::WPMPolarity { wpm: 20, polarity: true },
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                        Frame::KeyingPerfectDit,
                    ]);
                    assert_eq!(vec, expected_encoding);
                    assert_eq!(encoding.is_end, false); // It'll be set in the overflow block, next.
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
            // Block 2
            match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(encoding) => {
                    info!("Received SourceEncoding of {}", encoding);
                    let vec = encoding.block;
                    let expected_encoding = encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
                        Frame::KeyingEnd,
                    ]);
                    assert_eq!(vec, expected_encoding);
                    assert_eq!(encoding.is_end, true);
                }
                Err(e) => {
                    panic!("Should have received a SourceEncoding, not an error of {}", e);
                }
            }
        });
    }


    #[rstest]
    fn all_types_of_keying(mut fixture: SourceEncoderFixture) {
        test_util::panic_after(Duration::from_secs(2), move || {
            fixture.source_encoder.set_keyer_speed(20);
            test_util::wait_5_ms();

            fixture.keying_event_tx.broadcast(KeyingEvent::Start());
            // Perfects
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 60 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 180 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 420 }));
            // Deltas
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 65 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 175 }));
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: false, duration: 425 }));
            // Naïve
            fixture.keying_event_tx.broadcast(KeyingEvent::Timed(KeyingTimedEvent { up: true, duration: 600 }));

            fixture.keying_event_tx.broadcast(KeyingEvent::End());

            test_util::wait_5_ms();
            fixture.source_encoder.emit();
            test_util::wait_5_ms();

            // Block 1
            expect_encoded_block(&mut fixture, encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
                Frame::WPMPolarity { wpm: 20, polarity: true },
                Frame::KeyingPerfectDit,
                Frame::KeyingPerfectDah,
                Frame::KeyingPerfectWordgap,
                Frame::KeyingDeltaDit { delta: 5 },
                Frame::KeyingDeltaDah { delta: -5 },
                Frame::KeyingDeltaWordgap { delta: 5 },
            ]));

            // Block 2
            expect_encoded_block(&mut fixture, encoded(TEST_SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, 20, &[
                Frame::WPMPolarity { wpm: 20, polarity: true },
                Frame::KeyingNaive { duration: 600 },
                Frame::KeyingEnd,
            ]));
        });
    }

    fn expect_encoded_block(fixture: &mut SourceEncoderFixture, expected_encoding: Vec<u8>) {
        match fixture.source_encoder_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(encoding) => {
                info!("Received SourceEncoding of {}", encoding);
                let vec = encoding.block;

                assert_eq!(vec, expected_encoding);
            }
            Err(e) => {
                panic!("Should have received a SourceEncoding, not an error of {}", e);
            }
        }
    }
}
