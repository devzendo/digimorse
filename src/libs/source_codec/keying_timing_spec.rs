
#[cfg(test)]
mod keying_timing_spec {
    use rstest::*;
    use std::env;
    use crate::libs::source_codec::keying_timing::{dah_encoding_range, DefaultKeyingTiming, dit_encoding_range, KeyingTiming, wordgap_encoding_range};

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct KeyingTimingFixture {
        timing: Box<dyn KeyingTiming>,
    }

    // impl KeyingEncoderFixture {
    // }

    #[fixture]
    fn fixture() -> KeyingTimingFixture {
        let mut timing = Box::new(DefaultKeyingTiming::new());
        timing.set_keyer_speed(20);
        KeyingTimingFixture {
            timing: timing,
        }
    }

    // Encoding durations --------------------------------------------------------------------------

    // For WPMs that don't yield integer durations...
    #[rstest]
    fn perfect_durations_floor_correctly(mut fixture: KeyingTimingFixture) {
        fixture.timing.set_keyer_speed(7);
        assert_eq!(fixture.timing.get_perfect_dit_ms(), 171);
        assert_eq!(fixture.timing.get_perfect_dah_ms(), 514);
        assert_eq!(fixture.timing.get_perfect_wordgap_ms(), 1200);

        fixture.timing.set_keyer_speed(33);
        assert_eq!(fixture.timing.get_perfect_dit_ms(), 36);
        assert_eq!(fixture.timing.get_perfect_dah_ms(), 109);
        assert_eq!(fixture.timing.get_perfect_wordgap_ms(), 254);

        fixture.timing.set_keyer_speed(39);
        assert_eq!(fixture.timing.get_perfect_dit_ms(), 30);
        assert_eq!(fixture.timing.get_perfect_dah_ms(), 92);
        assert_eq!(fixture.timing.get_perfect_wordgap_ms(), 215);
    }

    // For WPMs that don't yield integer durations...
    #[rstest]
    fn delta_ranges_are_correct_for_the_wpm(mut fixture: KeyingTimingFixture) {
        // reset
        fixture.timing.set_keyer_speed(0);
        assert_eq!(fixture.timing.get_dit_delta_range(), (0, 0));
        assert_eq!(fixture.timing.get_dah_delta_range(), (0, 0));
        assert_eq!(fixture.timing.get_wordgap_delta_range(), (0, 0));

        // range of speeds
        fixture.timing.set_keyer_speed(5);
        assert_eq!(fixture.timing.get_dit_delta_range(), (-240, 240));
        assert_eq!(fixture.timing.get_dah_delta_range(), (-239, 479));
        assert_eq!(fixture.timing.get_wordgap_delta_range(), (-480, 367));

        fixture.timing.set_keyer_speed(20);
        assert_eq!(fixture.timing.get_dit_delta_range(), (-60, 60));
        assert_eq!(fixture.timing.get_dah_delta_range(), (-59, 119));
        assert_eq!(fixture.timing.get_wordgap_delta_range(), (-120, 120));

        fixture.timing.set_keyer_speed(60);
        assert_eq!(fixture.timing.get_dit_delta_range(), (-20, 20));
        assert_eq!(fixture.timing.get_dah_delta_range(), (-19, 39));
        assert_eq!(fixture.timing.get_wordgap_delta_range(), (-40, 40));
    }

    #[rstest]
    fn delta_ranges_floor_correctly(mut fixture: KeyingTimingFixture) {
        fixture.timing.set_keyer_speed(7);
        assert_eq!(fixture.timing.get_dit_delta_range(), (-171, 171));
        assert_eq!(fixture.timing.get_dah_delta_range(), (-170, 341));
        assert_eq!(fixture.timing.get_wordgap_delta_range(), (-342, 342));

        fixture.timing.set_keyer_speed(33);
        assert_eq!(fixture.timing.get_dit_delta_range(), (-36, 36));
        assert_eq!(fixture.timing.get_dah_delta_range(), (-35, 71));
        assert_eq!(fixture.timing.get_wordgap_delta_range(), (-72, 72));

        fixture.timing.set_keyer_speed(39);
        assert_eq!(fixture.timing.get_dit_delta_range(), (-30, 30));
        assert_eq!(fixture.timing.get_dah_delta_range(), (-29, 60));
        assert_eq!(fixture.timing.get_wordgap_delta_range(), (-61, 61));
    }

    // Encoding ranges -----------------------------------------------------------------------------

    #[test]
    #[should_panic]
    pub fn dit_encoding_range_at_zero() {
        dit_encoding_range(0);
    }

    #[test]
    #[should_panic]
    pub fn dit_encoding_range_at_4() {
        dit_encoding_range(4);
    }

    #[test]
    #[should_panic]
    pub fn dit_encoding_range_at_61() {
        dit_encoding_range(61);
    }

    #[test]
    #[should_panic]
    pub fn dah_encoding_range_at_zero() {
        dah_encoding_range(0);
    }

    #[test]
    #[should_panic]
    pub fn dah_encoding_range_at_4() {
        dah_encoding_range(4);
    }

    #[test]
    #[should_panic]
    pub fn dah_encoding_range_at_61() {
        dah_encoding_range(61);
    }

    #[test]
    #[should_panic]
    pub fn wordgap_encoding_range_at_zero() {
        wordgap_encoding_range(0);
    }

    #[test]
    #[should_panic]
    pub fn wordgap_encoding_range_at_4() {
        wordgap_encoding_range(4);
    }

    #[test]
    #[should_panic]
    pub fn wordgap_encoding_range_at_61() {
        wordgap_encoding_range(61);
    }

    #[test]
    pub fn encoding_ranges_at_boundaries() {
        assert_eq!(dit_encoding_range(5), (8, 8));
        assert_eq!(dit_encoding_range(9), (8, 8));
        assert_eq!(dit_encoding_range(10), (7, 7));
        assert_eq!(dit_encoding_range(18), (7, 7));
        assert_eq!(dit_encoding_range(19), (6, 6));
        assert_eq!(dit_encoding_range(37), (6, 6));
        assert_eq!(dit_encoding_range(38), (5, 5));
        assert_eq!(dit_encoding_range(60), (5, 5));

        assert_eq!(dah_encoding_range(5), (8, 9));
        assert_eq!(dah_encoding_range(9), (8, 9));
        assert_eq!(dah_encoding_range(10), (7, 8));
        assert_eq!(dah_encoding_range(18), (7, 8));
        assert_eq!(dah_encoding_range(19), (6, 7));
        assert_eq!(dah_encoding_range(37), (6, 7));
        assert_eq!(dah_encoding_range(38), (5, 6));
        assert_eq!(dah_encoding_range(60), (5, 6));

        assert_eq!(wordgap_encoding_range(5), (9, 9));
        assert_eq!(wordgap_encoding_range(9), (9, 9));
        assert_eq!(wordgap_encoding_range(10), (8, 8));
        assert_eq!(wordgap_encoding_range(18), (8, 8));
        assert_eq!(wordgap_encoding_range(19), (7, 7));
        assert_eq!(wordgap_encoding_range(37), (7, 7));
        assert_eq!(wordgap_encoding_range(38), (6, 6));
        assert_eq!(wordgap_encoding_range(60), (6, 6));
    }
}