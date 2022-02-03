extern crate hamcrest2;

#[cfg(test)]
mod application_spec {
    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use bus::{Bus, BusReader};
    use hamcrest2::prelude::*;
    use log::{debug, info};
    use rstest::*;
    use hamcrest2::prelude::*;
    use syncbox::ScheduledThreadPool;
    use crate::libs::application::application::{Application, Mode};

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

    pub struct ApplicationFixture {
        terminate: Arc<AtomicBool>,
        scheduled_thread_pool: Arc<ScheduledThreadPool>,
        application: Application,
    }

    #[fixture]
    fn fixture() -> ApplicationFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());

        let application = Application::new(terminate.clone(), scheduled_thread_pool.clone());

        info!("Fixture setup sleeping");
        test_util::wait_5_ms(); // give things time to start
        info!("Fixture setup out of sleep");

        ApplicationFixture {
            terminate,
            scheduled_thread_pool,
            application,
        }
    }

    impl Drop for ApplicationFixture {
        fn drop(&mut self) {
            debug!("ApplicationFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("ApplicationFixture ...set terminate flag");
        }
    }

    #[rstest]
    pub fn termination(mut fixture: ApplicationFixture) {
        assert_eq!(fixture.application.terminated(), false);
        test_util::wait_5_ms();
        fixture.application.terminate();
        test_util::wait_5_ms();
        assert_eq!(fixture.application.terminated(), true);
        assert_eq!(fixture.terminate.load(Ordering::SeqCst), true);
    }

    #[rstest]
    pub fn initial_mode(fixture: ApplicationFixture) {
        assert_that!(fixture.application.get_mode(), none());
    }

    #[rstest]
    pub fn mode_keyer_diag(mut fixture: ApplicationFixture) {
        fixture.application.set_mode(Mode::KeyerDiag);
        assert_that!(fixture.application.get_mode(), has(Mode::KeyerDiag));
    }

}
