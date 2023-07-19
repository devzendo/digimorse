#[cfg(test)]
mod fft_spec {
    use std::cell::RefCell;
    use std::env;
    use std::f32::consts::PI;
    use std::sync::{Arc, Mutex};
    use hamcrest2::prelude::*;
    use rstest::*;
    use crate::libs::buffer_pool::observable_buffer::{OBSERVABLE_BUFFER_SLICE_SIZE, ObservableBufferSlice};
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
        fft_slices: RefCell<Vec<ObservableFrequencySlice>>,
        fft_observer: Arc<Mutex<FFTingBufferObserver>>,
    }

    impl Observer<ObservableFrequencySlice> for FFTFixture {
        fn on_notify(&self, observable: &ObservableFrequencySlice) {
            self.fft_slices.borrow_mut().push(observable.clone());
        }
    }

    #[fixture]
    fn fixture() -> Arc<FFTFixture> {
        let fft_observer = Arc::new(Mutex::new(FFTingBufferObserver::new()));
        let mut fixture = Arc::new(FFTFixture {
            fft_slices: RefCell::new(vec![]),
            fft_observer,
        });
        fixture.fft_observer.lock().unwrap().add_observer(fixture.clone() as Arc<dyn Observer<ObservableFrequencySlice>>);
        fixture.clone()
    }

    impl Drop for FFTFixture {
        fn drop(&mut self) {
        }
    }

    #[rstest]
    #[should_panic]
    pub fn wrong_size_buffer_panics(fixture: Arc<FFTFixture>) {
        let samples: Vec<f32> = vec![];
        let buffer = ObservableBufferSlice { slice: samples };
        fixture.fft_observer.lock().unwrap().on_notify(&buffer);
    }

    #[rstest]
    #[serial]
    pub fn buffer_in_fft_out(fixture: Arc<FFTFixture>) {
        let mut samples: Vec<f32> = vec![];
        // A 600Hz sine wave..
        let freq: f32 = 600.0;
        let sample_rate = 48000.0;
        let delta_phase = 2.0_f32 * PI * freq / sample_rate;
        let mut phase = 0.0;
        for idx in 0..OBSERVABLE_BUFFER_SLICE_SIZE {
            let sine_val = f32::sin(phase);
            samples.push(sine_val);
            phase += delta_phase;
        }
        let buffer = ObservableBufferSlice { slice: samples };
        fixture.fft_observer.lock().unwrap().on_notify(&buffer);
        assert_that!(fixture.fft_slices.borrow().len(), equal_to(1));
    }
}
