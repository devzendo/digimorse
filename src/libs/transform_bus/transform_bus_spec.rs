extern crate hamcrest2;

#[cfg(test)]
mod transform_bus_spec {
    use std::env;
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use bus::{Bus, BusReader};
    use hamcrest2::prelude::*;
    use log::{debug, info};
    use rstest::*;
    use crate::libs::transform_bus::transform_bus::TransformBus;

    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct TransformBusFixture {
        terminate: Arc<AtomicBool>,
        input_tx: Bus<String>,
        // Not read, but needs storing to maintain lifetime
        _transform_bus: Arc<Mutex<TransformBus<String, (String, usize)>>>,
        output_rx: BusReader<(String, usize)>,
    }

    static mut COUNT: usize = 0;

    fn transform(strin: String) -> (String, usize) {
        unsafe {
            COUNT += 1;
            return (strin, COUNT);
        }
    }

    #[fixture]
    fn fixture() -> TransformBusFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let mut input_tx: Bus<String> = Bus::new(16);
        let input_rx = input_tx.add_rx();
        let output_tx: Bus<(String, usize)> = Bus::new(16); // TODO the output_tx is going to need to be shared with other writers, so will need to be Arc<Mutex<Bus<(String, usize)>>> or somesuch
        let transform_bus = TransformBus::new(input_rx, output_tx, transform, terminate.clone());
        let arc_transform_bus = Arc::new(Mutex::new(transform_bus));
        let output_rx = arc_transform_bus.lock().unwrap().add_reader();
        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        TransformBusFixture {
            terminate,
            input_tx,
            _transform_bus: arc_transform_bus,
            output_rx,
        }
    }

    impl Drop for TransformBusFixture {
        fn drop(&mut self) {
            debug!("TransformBusFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("TransformBusFixture ...set terminate flag");
        }
    }

    #[rstest]
    pub fn transform_messages(mut fixture: TransformBusFixture) {
        debug!("Sending a message in...");
        fixture.input_tx.broadcast("First".to_owned());
        fixture.input_tx.broadcast("Second".to_owned());
        fixture.input_tx.broadcast("Third".to_owned());
        debug!("Waiting for output...");
        expect_recv(&mut fixture, ("First".to_owned(), 1));
        expect_recv(&mut fixture, ("Second".to_owned(), 2));
        expect_recv(&mut fixture, ("Third".to_owned(), 3));
    }

    fn expect_recv(fixture: &mut TransformBusFixture, expected: (String, usize)) {
        match fixture.output_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(item) => {
                debug!("Output: {:?}", item);
                assert_that!(item.0, equal_to(expected.0));
                assert_that!(item.1, equal_to(expected.1));
            }
            Err(_) => {
                panic!("timeout");
            }
        }
    }
}
