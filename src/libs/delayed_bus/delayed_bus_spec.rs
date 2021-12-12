extern crate hamcrest2;

#[cfg(test)]
mod delayed_bus_spec {
    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use bus::{Bus, BusReader};
    use hamcrest2::prelude::*;
    use log::{debug, info};
    use rstest::*;
    use crate::libs::delayed_bus::delayed_bus::DelayedBus;

    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct DelayedBusFixture {
        terminate: Arc<AtomicBool>,
        input_tx: Bus<String>,
        output_rx: BusReader<String>,
        delayed_bus: DelayedBus<String>,
    }

    #[fixture]
    fn fixture() -> DelayedBusFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut input_tx = Bus::new(16);
        let input_rx = input_tx.add_rx();
        let mut output_tx = Bus::new(16);
        let output_rx = output_tx.add_rx();
        let delayed_bus = DelayedBus::new(input_rx, output_tx, terminate.clone(), Duration::from_millis(250));

        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        DelayedBusFixture {
            terminate,
            input_tx,
            output_rx,
            delayed_bus
        }
    }

    impl Drop for DelayedBusFixture {
        fn drop(&mut self) {
            debug!("DelayedBusFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("DelayedBusFixture ...set terminate flag");
        }
    }

    #[test]
    fn hold_the_bus() {
    }
}
