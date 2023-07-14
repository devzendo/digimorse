#[cfg(test)]
mod fft_spec {
    use std::cell::RefCell;
    use std::env;
    use std::sync::Arc;

    use hamcrest2::prelude::*;
    use rstest::*;
    use crate::libs::buffer_pool::observable_buffer::ObservableBufferSlice;
    use crate::libs::patterns::observer::Observer;
    use crate::libs::receiver::fft::{FFTingBufferObserver, ObservableFrequencySlice};

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct FFTFixture {
        fft_slices: Vec<ObservableFrequencySlice>,
        fft_observer: Arc<dyn Observer<ObservableBufferSlice<f32>>>,
    }

    impl Observer<ObservableFrequencySlice> for FFTFixture {
        fn on_notify(&mut self, observable: &ObservableFrequencySlice) {
            self.fft_slices.push(observable.clone());
        }
    }

    #[fixture]
    fn fixture() -> Arc<FFTFixture> {
        let fft_observer = Arc::new(FFTingBufferObserver::new());
        let mut fixture = Arc::new(FFTFixture {
            fft_slices: vec![],
            fft_observer,
        });
        fixture.fft_observer.add_observer(fixture.clone());
        fixture.clone()
    }

    impl Drop for FFTFixture {
        fn drop(&mut self) {
        }
    }

    #[rstest]
    #[serial]
    pub fn buffer_in_fft_out(fixture: Arc<FFTFixture>) {
        // TBC...

    }
}
