extern crate hamcrest2;

#[cfg(test)]
mod source_decoder_spec {
    use crate::libs::keyer_io::keyer_io::{KeyingEvent, KeyerSpeed, KeyingTimedEvent};
    use crate::libs::source_codec::source_encoder::SourceEncoder;
    use crate::libs::source_codec::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};
    use bus::{Bus, BusReader};
    use log::{debug, info};
    use hamcrest2::prelude::*;
    use rstest::*;
    use std::{env, thread};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use crate::libs::source_codec::test_encoding_builder::{encoded, Frame};
    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}
}
