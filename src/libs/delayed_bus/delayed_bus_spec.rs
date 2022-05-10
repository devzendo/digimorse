extern crate hamcrest2;

#[cfg(test)]
mod delayed_bus_spec {
    use rand::Rng;
    use std::{env, thread};
    use std::sync::{Arc, Mutex};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    use bus::{Bus, BusReader};
    use hamcrest2::prelude::*;
    use log::{debug, info};
    use rstest::*;
    use crate::libs::application::application::{BusInput, BusOutput};
    use crate::libs::delayed_bus::delayed_bus::DelayedBus;

    use crate::libs::util::test_util;
    use crate::libs::util::test_util::wait_n_ms;
    use crate::libs::util::util::get_epoch_ms;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct DelayedBusFixture {
        terminate: Arc<AtomicBool>,
        input_tx: Arc<Mutex<Bus<u128>>>,
        output_rx: BusReader<u128>,
        _delayed_bus: DelayedBus<u128>,
    }

    const DELAY_MILLIS: u64 = 250;

    #[fixture]
    fn fixture() -> DelayedBusFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let input_tx = Arc::new(Mutex::new(Bus::new(16)));
        let input_rx = input_tx.lock().unwrap().add_rx();
        let mut output_tx = Bus::new(16);
        let output_rx = output_tx.add_rx();
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

        let mut delayed_bus = DelayedBus::new(terminate.clone(), scheduled_thread_pool, Duration::from_millis(DELAY_MILLIS));
        delayed_bus.set_input_rx(Arc::new(Mutex::new(input_rx)));
        delayed_bus.set_output_tx(Arc::new(Mutex::new(output_tx)));

        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        DelayedBusFixture {
            terminate,
            input_tx,
            output_rx,
            _delayed_bus: delayed_bus
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

    #[rstest]
    pub fn hold_the_bus(mut fixture: DelayedBusFixture) {
        debug!("Sending a message in...");
        fixture.input_tx.lock().unwrap().broadcast(get_epoch_ms());
        debug!("Waiting for output...");
        loop {
            match fixture.output_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(item) => {
                    let now = get_epoch_ms();
                    debug!("Output: {}", item);
                    let duration = (now - item) as u64;
                    debug!("Item was in delayed bus for {} ms - was emitted {} ms after the configured delay", duration, duration - DELAY_MILLIS);
                    assert_that!(duration, greater_than_or_equal_to(DELAY_MILLIS));
                    assert_that!(duration, less_than(DELAY_MILLIS + 100));
                    break;
                }
                Err(_) => {
                    debug!("timeout");
                }
            }
        }
    }

    #[rstest]
    pub fn ordering_is_maintained(mut fixture: DelayedBusFixture) {
        debug!("Sending messages in...");
        let bus = fixture.input_tx.clone();
        thread::spawn(move || {
            for i in 1..50 {
                let num = rand::thread_rng().gen_range(0..50);
                wait_n_ms(num);
                bus.lock().unwrap().broadcast(i);
            }
        });
        debug!("Waiting for output...");
        let mut expected = 1;
        loop {
            match fixture.output_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(item) => {
                    let received = item as u64;
                    debug!("Output: {}", received);
                    assert_that!(received, equal_to(expected));
                    expected += 1;
                    if received == 49 {
                        debug!("Final value received");
                        break;
                    }
                }
                Err(_) => {
                    debug!("timeout");
                }
            }
        }
    }
}
