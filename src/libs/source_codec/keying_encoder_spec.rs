use crate::libs::keyer_io::keyer_io::KeyerEdgeDurationMs;

const PERFECT_DIT_DURATION: KeyerEdgeDurationMs = 60;
const PERFECT_DAH_DURATION: KeyerEdgeDurationMs = 180;
const PERFECT_WORDGAP_DURATION: KeyerEdgeDurationMs = 420;

#[cfg(test)]
mod keying_encoder_spec {
    use rstest::*;
    use std::env;
    use std::sync::{Arc, RwLock};
    use crate::libs::keyer_io::keyer_io::KeyingTimedEvent;
    use crate::libs::source_codec::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
    use crate::libs::source_codec::keying_encoder::{DefaultKeyingEncoder, KeyingEncoder};
    use crate::libs::source_codec::keying_encoder::keying_encoder_spec::{PERFECT_DAH_DURATION, PERFECT_DIT_DURATION, PERFECT_WORDGAP_DURATION};
    use crate::libs::source_codec::source_encoding::SourceEncodingBuilder;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct KeyingEncoderFixture {
        storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>,
        encoder: Box<dyn KeyingEncoder>,
    }

    impl KeyingEncoderFixture {
        pub fn bytes(&mut self) -> Vec<u8> {
            self.storage.write().unwrap().build().block
        }
    }

    #[fixture]
    fn fixture() -> KeyingEncoderFixture {
        let storage: Box<dyn SourceEncodingBuilder + Send + Sync> = Box::new
            (BitvecSourceEncodingBuilder::new());
        let arc_storage = Arc::new(RwLock::new(storage));
        let mut encoder = Box::new(DefaultKeyingEncoder::new(arc_storage.clone()));
        encoder.set_keyer_speed(20);
        KeyingEncoderFixture {
            storage: arc_storage.clone(),
            encoder,
        }
    }

    #[rstest]
    #[should_panic]
    pub fn panic_on_encode_with_no_speed_set(mut fixture: KeyingEncoderFixture) {
        // For convenience, the fixture() sets the speed to 20WPM. Unset it, as though it had not
        // been set...
        fixture.encoder.set_keyer_speed(0);

        fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION });
    }

    #[rstest]
    pub fn encode_perfect_dit(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.bytes(), vec![0b01100000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_perfect_dah(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DAH_DURATION }), true);
        assert_eq!(fixture.bytes(), vec![0b01110000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn encode_perfect_wordgap(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_WORDGAP_DURATION }), true);
        assert_eq!(fixture.bytes(), vec![0b10000000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    fn keying_wont_fit_in_block_so_returns_false(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), true);
        // This one won't fit in the block..
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: PERFECT_DIT_DURATION }), false);
    }

    #[rstest]
    fn wpm_changes_are_tracked_for_perfect_encodings(mut fixture: KeyingEncoderFixture) {
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: true, duration: PERFECT_DIT_DURATION }), true);
        fixture.encoder.set_keyer_speed(40);
        assert_eq!(fixture.encoder.encode_keying(&KeyingTimedEvent { up: false, duration: 30 }), true);
        // No WPM|Polarity encoding at the start or after the change of speed, but the correct
        // perfect duration is updated.
        assert_eq!(fixture.bytes(), vec![0b01100110, 0, 0, 0, 0, 0, 0, 0]);
    }
}
