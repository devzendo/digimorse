#[cfg(test)]
mod fft_spec {
    use crate::libs::buffer_pool::observable_buffer::{
        ObservableBufferSlice, OBSERVABLE_BUFFER_SLICE_SIZE,
    };
    use crate::libs::patterns::observer::Observer;
    use crate::libs::receiver::fft::{FFTingBufferObserver, ObservableFrequencySlice};
    use crate::libs::util::graph::plot_graph;
    use hamcrest2::prelude::*;
    use rstest::*;
    use std::cell::RefCell;
    use std::env;
    use std::f32::consts::PI;
    use std::sync::{Arc, Mutex};

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
        let fixture = Arc::new(FFTFixture {
            fft_slices: RefCell::new(vec![]),
            fft_observer,
        });
        fixture
            .fft_observer
            .lock()
            .unwrap()
            .add_observer(fixture.clone() as Arc<dyn Observer<ObservableFrequencySlice>>);
        fixture.clone()
    }

    impl Drop for FFTFixture {
        fn drop(&mut self) {}
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
        // The buffer holding FFT output data is initially empty.
        assert_that!(fixture.fft_slices.borrow().len(), equal_to(0));

        let mut samples: Vec<f32> = vec![];
        // A 600Hz sine wave..
        let freq: f32 = 600.0;
        let sample_rate = 48000.0;
        let delta_phase = 2.0_f32 * PI * freq / sample_rate;
        let mut phase = 0.0;
        for _idx in 0..OBSERVABLE_BUFFER_SLICE_SIZE {
            let sine_val = f32::sin(phase);
            samples.push(sine_val);
            phase += delta_phase;
        }

        plot_graph("./six-hundred-hz.png", "600 Hz sinusoidal waveform", &samples, 0, OBSERVABLE_BUFFER_SLICE_SIZE, -1.0, 1.0);

        let buffer = ObservableBufferSlice { slice: samples };
        fixture.fft_observer.lock().unwrap().on_notify(&buffer);
        let reals = fixture.fft_slices.borrow();
        // assert_that!(reals.len(), equal_to(1));
        let fft = reals.get(0).unwrap();

        plot_graph("./six-hundred-hz-fft.png", "FFT of 600 Hz sinusoidal waveform", &fft.reals.as_ref(), 0, fft.reals.len(), -5.0, 5.0);

        // From index 0 to 18, value is ~ 0. There's a peak around index 24:
        // 19, 0.0026358399
        // 20, 0.003137786
        // 21, 0.0054621343
        // 22, 0.0057048774
        // 23, 0.01720088
        // 24, 30.968023
        // 25, 0.017700404
        // 26, 0.006294353
        // 27, 0.006056759
        // 28, 0.0037250596
        // 29, 0.003235286
        for i in 0..fft.reals.len() {
            println!("{}, {}", i, fft.reals[i]);
            if i == 24 {
                assert_that!(fft.reals[i], greater_than(10.0));
            } else {
                assert_that!(fft.reals[i], less_than(0.5));
            }
        }
        // 37.5Hz => A spread of high values peaking at index 1
        // 75Hz => index 3
        // 150Hz => index 6
        // 300Hz => index 12
        // 600Hz => index 24
        // 1200Hz => index 48
        // 2400Hz => index 96
        // 4800Hz => index 192
        // Ok it's linear.. index = Hz/25.
        // 24000Hz => A spread of high values peaking at index 960 (the end)
    }
}
