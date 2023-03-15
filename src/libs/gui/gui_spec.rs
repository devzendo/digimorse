#[cfg(test)]
mod guis_spec {
    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use log::{debug, info};
    use portaudio as pa;
    use rstest::*;
    use syncbox::ScheduledThreadPool;
    use crate::libs::application::application::{Application, ApplicationMode};
    use crate::libs::config_dir::config_dir;
    use crate::libs::config_file::config_file::ConfigurationStore;
    use crate::libs::gui::gui;

    use crate::libs::util::test_util;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct GuiFixture {
        terminate: Arc<AtomicBool>,
        scheduled_thread_pool: Arc<ScheduledThreadPool>,
    }


    #[fixture]
    fn fixture() -> GuiFixture {
        let terminate = Arc::new(AtomicBool::new(false));
        let scheduled_thread_pool = Arc::new(syncbox::ScheduledThreadPool::single_thread());


        info!("Fixture setup sleeping");
        test_util::wait_5_ms();
        // give things time to start
        info!("Fixture setup out of sleep");

        GuiFixture {
            terminate,
            scheduled_thread_pool
        }
    }

    impl Drop for GuiFixture {
        fn drop(&mut self) {
            debug!("GuiFixture setting terminate flag...");
            self.terminate.store(true, Ordering::SeqCst);
            test_util::wait_5_ms();
            debug!("GuiFixture ...set terminate flag");
        }
    }

    // Manually invoked
    #[rstest]
    #[ignore]
    pub fn launch_gui(fixture: GuiFixture) {
        let home_dir = dirs::home_dir();
        let config_path = config_dir::configuration_directory(home_dir).unwrap();
        let mut config = ConfigurationStore::new(config_path).unwrap();

        let pa = pa::PortAudio::new().unwrap();
        let mut application = Application::new(fixture.terminate.clone(), fixture.scheduled_thread_pool.clone(), pa);
        application.set_ctrlc_handler();
        application.set_mode(ApplicationMode::Full);

        gui::initialise(&mut config, &mut application);

    }
}