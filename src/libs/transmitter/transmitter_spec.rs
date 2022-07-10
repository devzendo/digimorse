extern crate hamcrest2;

#[cfg(test)]
mod transmitter_spec {
    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use log::{debug, info};
    use rstest::*;

    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct TransmitterFixture {
        terminate: Arc<AtomicBool>,
    }

    #[fixture]
    fn fixture() -> TransmitterFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        info!("Fixture setup sleeping");
        test_util::wait_5_ms();
        // give things time to start
        info!("Fixture setup out of sleep");

        TransmitterFixture {
            terminate,
        }
    }

    impl Drop for TransmitterFixture {
        fn drop(&mut self) {
            debug!("TransmitterFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("TransmitterFixture ...set terminate flag");
        }
    }

    #[rstest]
    pub fn do_something(_fixture: TransmitterFixture) {}
}
