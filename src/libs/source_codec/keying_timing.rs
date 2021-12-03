use std::cmp::min;
use log::debug;
use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyerSpeed};

pub type KeyerRangeDelta = i16;

/// All calculated durations, ranges of deltas from perfect timing, bounds of keying durations for
/// the three elements, numbers of bits required to encode deltas - all related to, and calculated
/// from the current keyer speed.
pub trait KeyingTiming {
    // The KeyingEncoder needs to know the keyer speed to build keying frames into their most
    // compact form; a minimal delta from the three timing elements.
    fn set_keyer_speed(&mut self, speed: KeyerSpeed);
    fn get_keyer_speed(&self) -> KeyerSpeed;

    /// Obtain the perfect timings, for the current keyer speed.
    fn get_perfect_dit_ms(&self) -> KeyerEdgeDurationMs;
    fn get_perfect_dah_ms(&self) -> KeyerEdgeDurationMs;
    fn get_perfect_wordgap_ms(&self) -> KeyerEdgeDurationMs;

    /// Obtain the delta ranges, for the current keyer speed.
    fn get_dit_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);
    fn get_dah_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);
    fn get_wordgap_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta);

    fn get_lower_dit_bound(&self) -> KeyerEdgeDurationMs;
    fn get_upper_dit_bound(&self) -> KeyerEdgeDurationMs;
    fn get_lower_dah_bound(&self) -> KeyerEdgeDurationMs;
    fn get_upper_dah_bound(&self) -> KeyerEdgeDurationMs;
    fn get_lower_wordgap_bound(&self) -> KeyerEdgeDurationMs;
    fn get_upper_wordgap_bound(&self) -> KeyerEdgeDurationMs;

    fn dit_encoding_range(&self) -> (usize, usize);
    fn dah_encoding_range(&self) -> (usize, usize);
    fn wordgap_encoding_range(&self) -> (usize, usize);
}

pub struct DefaultKeyingTiming {
    keyer_speed: KeyerSpeed,

    perfect_dit_ms: KeyerEdgeDurationMs,
    perfect_dah_ms: KeyerEdgeDurationMs,
    perfect_wordgap_ms: KeyerEdgeDurationMs,
    negative_dit_range: KeyerRangeDelta,
    positive_dit_range: KeyerRangeDelta,
    lower_dit_bound: KeyerEdgeDurationMs,
    upper_dit_bound: KeyerEdgeDurationMs,
    negative_dah_range: KeyerRangeDelta,
    positive_dah_range: KeyerRangeDelta,
    lower_dah_bound: KeyerEdgeDurationMs,
    upper_dah_bound: KeyerEdgeDurationMs,
    negative_wordgap_range: KeyerRangeDelta,
    positive_wordgap_range: KeyerRangeDelta,
    lower_wordgap_bound: KeyerEdgeDurationMs,
    upper_wordgap_bound: KeyerEdgeDurationMs,
}

impl DefaultKeyingTiming {
    pub fn new() -> Self {
        Self {
            keyer_speed: 0,
            perfect_dit_ms: 0,
            perfect_dah_ms: 0,
            perfect_wordgap_ms: 0,
            negative_dit_range: 0,
            positive_dit_range: 0,
            lower_dit_bound: 0,
            upper_dit_bound: 0,
            negative_dah_range: 0,
            positive_dah_range: 0,
            lower_dah_bound: 0,
            upper_dah_bound: 0,
            negative_wordgap_range: 0,
            positive_wordgap_range: 0,
            lower_wordgap_bound: 0,
            upper_wordgap_bound: 0
        }
    }
}

impl KeyingTiming for DefaultKeyingTiming {
    fn set_keyer_speed(&mut self, speed: KeyerSpeed) {
        self.keyer_speed = speed;
        // Allow setting it to zero to aid tests
        if self.keyer_speed == 0 {
            self.perfect_dit_ms = 0;
            self.perfect_dah_ms = 0;
            self.perfect_wordgap_ms = 0;
            self.negative_dit_range = 0;
            self.positive_dit_range = 0;
            self.negative_dah_range = 0;
            self.positive_dah_range = 0;
            self.negative_wordgap_range = 0;
            self.positive_wordgap_range = 0;

            self.lower_dit_bound = 0;
            self.upper_dit_bound = 0;
            self.lower_dah_bound = 0;
            self.upper_dah_bound = 0;
            self.lower_wordgap_bound = 0;
            self.upper_wordgap_bound = 0;
        } else {
            let decimal_dit_ms = 1200f32 / speed as f32;
            let decimal_dah_ms = decimal_dit_ms * 3f32;
            let decimal_wordgap_ms = decimal_dit_ms * 7f32;
            //debug!("decimal_dit_ms is {}", decimal_dit_ms);
            let dit = decimal_dit_ms as u16; // funky...
            self.perfect_dit_ms = decimal_dit_ms as KeyerEdgeDurationMs;
            self.perfect_dah_ms = decimal_dah_ms as KeyerEdgeDurationMs;
            self.perfect_wordgap_ms = decimal_wordgap_ms as KeyerEdgeDurationMs;
            // Delta ranges are based off midpoints between the perfect dit/dah/wordgap. The maximum
            // is capped at 367, not 480 since wordgap+367=2047 which fits in 11 bits. Slow delta
            // wordgaps above 367 would be encoded as a naïve.
            // See docs/Morse speeds.xlsx for the derivations of these.
            let dit_dah_midpoint = decimal_dah_ms - decimal_dit_ms;
            let dah_wordgap_midpoint = decimal_dah_ms + ((decimal_wordgap_ms - decimal_dah_ms)/2f32);
            //debug!("dit_dah_midpoint {}, dah_wordgap_midpoint {}", dit_dah_midpoint, dah_wordgap_midpoint);
            // There will be non-null intersections between the three, so I've shrunk the middle dah
            // range on either end by one. Then the three ranges are disjoint. This diverges from
            // the docs/Morse speeds.xlsx spreadsheet.
            self.negative_dit_range = -(dit as i16);
            self.positive_dit_range = dit as i16;
            self.negative_dah_range = -(decimal_dah_ms - dit_dah_midpoint) as i16 + 1;
            self.positive_dah_range = (dah_wordgap_midpoint - decimal_dah_ms) as i16 - 1;
            self.negative_wordgap_range = -(decimal_wordgap_ms - dah_wordgap_midpoint) as i16;
            self.positive_wordgap_range = min(367, -(self.negative_wordgap_range));
            // Delta encoding bounds...
            let dit_i16 = self.perfect_dit_ms as i16;
            let dah_i16 = self.perfect_dah_ms as i16;
            let wordgap_i16 = self.perfect_wordgap_ms as i16;
            self.lower_dit_bound = (dit_i16 + self.negative_dit_range) as KeyerEdgeDurationMs;
            self.upper_dit_bound = (dit_i16 + self.positive_dit_range) as KeyerEdgeDurationMs;
            self.lower_dah_bound = (dah_i16 + self.negative_dah_range) as KeyerEdgeDurationMs;
            self.upper_dah_bound = (dah_i16 + self.positive_dah_range) as KeyerEdgeDurationMs;
            self.lower_wordgap_bound = (wordgap_i16 + self.negative_wordgap_range) as KeyerEdgeDurationMs;
            self.upper_wordgap_bound = (wordgap_i16 + self.positive_wordgap_range) as KeyerEdgeDurationMs;
        }
        debug!("KeyingTiming speed set to {} WPM; dit: {}ms dah: {}ms wordgap: {}ms", self
            .keyer_speed,
            self.perfect_dit_ms, self.perfect_dah_ms, self.perfect_wordgap_ms);
        debug!("Delta dit [{} .. {}] dah [{} .. {}], wordgap [{} .. {}]",
            self.negative_dit_range, self.positive_dit_range,
            self.negative_dah_range, self.positive_dah_range,
            self.negative_wordgap_range, self.positive_wordgap_range
        );
        debug!("Duration dit [{} .. {}]ms dah [{} .. {}]ms wordgap [{} .. {}]ms",
            self.lower_dit_bound, self.upper_dit_bound,
            self.lower_dah_bound, self.upper_dah_bound,
            self.lower_wordgap_bound, self.upper_wordgap_bound);
    }

    fn get_keyer_speed(&self) -> KeyerSpeed {
        self.keyer_speed
    }

    fn get_perfect_dit_ms(&self) -> KeyerEdgeDurationMs {
        self.perfect_dit_ms
    }

    fn get_perfect_dah_ms(&self) -> KeyerEdgeDurationMs {
        self.perfect_dah_ms
    }

    fn get_perfect_wordgap_ms(&self) -> KeyerEdgeDurationMs {
        self.perfect_wordgap_ms
    }

    fn get_dit_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta) {
        (self.negative_dit_range, self.positive_dit_range)
    }

    fn get_dah_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta) {
        (self.negative_dah_range, self.positive_dah_range)
    }

    fn get_wordgap_delta_range(&self) -> (KeyerRangeDelta, KeyerRangeDelta) {
        (self.negative_wordgap_range, self.positive_wordgap_range)
    }

    fn get_lower_dit_bound(&self) -> KeyerEdgeDurationMs {
        self.lower_dit_bound
    }

    fn get_upper_dit_bound(&self) -> KeyerEdgeDurationMs {
        self.upper_dit_bound
    }

    fn get_lower_dah_bound(&self) -> KeyerEdgeDurationMs {
        self.lower_dah_bound
    }

    fn get_upper_dah_bound(&self) -> KeyerEdgeDurationMs {
        self.upper_dah_bound
    }

    fn get_lower_wordgap_bound(&self) -> KeyerEdgeDurationMs {
        self.lower_wordgap_bound
    }

    fn get_upper_wordgap_bound(&self) -> KeyerEdgeDurationMs {
        self.upper_wordgap_bound
    }

    fn dit_encoding_range(&self) -> (usize, usize) {
        dit_encoding_range(self.keyer_speed)
    }

    fn dah_encoding_range(&self) -> (usize, usize) {
        dah_encoding_range(self.keyer_speed)
    }

    fn wordgap_encoding_range(&self) -> (usize, usize) {
        wordgap_encoding_range(self.keyer_speed)
    }
}

// From the table of delta encoding bit ranges per keying speed
pub fn dit_encoding_range(wpm: KeyerSpeed) -> (usize, usize) {
    if wpm >= 5 {
        if wpm <= 9 {
            return (8, 8);
        } else if wpm >= 10 && wpm <= 18 {
            return (7, 7);
        } else if wpm >= 19 && wpm <= 37 {
            return (6, 6);
        } else if wpm <= 60 {
            return (5, 5)
        }
    }
    panic!("WPM of {} is out of range in dit_encoding_range", wpm);
}

pub fn dah_encoding_range(wpm: KeyerSpeed) -> (usize, usize) {
    if wpm >= 5 {
        if wpm <= 9 {
            return (8, 9);
        } else if wpm >= 10 && wpm <= 18 {
            return (7, 8);
        } else if wpm >= 19 && wpm <= 37 {
            return (6, 7);
        } else if wpm <= 60 {
            return (5, 6)
        }
    }
    panic!("WPM of {} is out of range in dah_encoding_range", wpm);
}

pub fn wordgap_encoding_range(wpm: KeyerSpeed) -> (usize, usize) {
    if wpm >= 5 {
        if wpm <= 9 {
            return (9, 9);
        } else if wpm >= 10 && wpm <= 18 {
            return (8, 8);
        } else if wpm >= 19 && wpm <= 37 {
            return (7, 7);
        } else if wpm <= 60 {
            return (6, 6)
        }
    }
    panic!("WPM of {} is out of range in wordgap_encoding_range", wpm);
}


#[cfg(test)]
#[path = "./keying_timing_spec.rs"]
mod keying_timing_spec;
