use std::cell::RefCell;
use std::sync::Arc;
use num::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use crate::libs::buffer_pool::observable_buffer::{OBSERVABLE_BUFFER_SLICE_SIZE, ObservableBufferSlice};
use crate::libs::patterns::observer::{ConcreteObserverList, Observable, Observer, ObserverList};

#[derive(Clone)]
pub struct ObservableFrequencySlice {
    reals: Vec<f32>,
}

impl Observable for ObservableFrequencySlice {
}

pub struct FFTingBufferObserver {
    _fft: RealFftPlanner<f32>,
    r2c: Arc<dyn RealToComplex<f32>>,
    // RefCell for interior mutability since the on_notify method does not have &mut self
    spectrum: RefCell<Vec<Complex<f32>>>, // FFT bin
    observers: ConcreteObserverList<ObservableFrequencySlice>,
}

impl FFTingBufferObserver {
    pub fn new() -> Self {
        let mut fft = RealFftPlanner::<f32>::new();
        let nfft = OBSERVABLE_BUFFER_SLICE_SIZE;
        let r2c = fft.plan_fft_forward(nfft);
        // make a vector for storing the spectrum
        let spectrum = r2c.make_output_vec();

        Self {
            _fft: fft,
            r2c,
            spectrum: RefCell::new(spectrum),
            observers: ConcreteObserverList::new(),
        }
    }

    pub fn add_observer(&mut self, observer: Arc<dyn Observer<ObservableFrequencySlice>>) {
        self.observers.register_observer(observer);
    }
}

impl Observer<ObservableBufferSlice<f32>> for FFTingBufferObserver {
    // The FFT observer zero-pads each slice (which holds 160ms of downsampled audio) to 320ms,
    // transforms, and emits that to its observers.
    fn on_notify(&self, one_sixty_ms_downsampled_audio: &ObservableBufferSlice<f32>) {
        let slice_len = one_sixty_ms_downsampled_audio.slice.len();
        // info!("slice_len size is {}", slice_len);
        if slice_len != OBSERVABLE_BUFFER_SLICE_SIZE {
            panic!("Expecting sample buffers of length {} not {}", OBSERVABLE_BUFFER_SLICE_SIZE, slice_len)
        }

        // Not sure this normalisation is needed - it isn't in test data.
        // normalise to [-1.0, 1.0]...
        let max: f32 = one_sixty_ms_downsampled_audio.slice
            .iter()
            .fold(0.0, |i, x| if *x > i { *x } else { i });
        let min: f32 = one_sixty_ms_downsampled_audio.slice
            .iter()
            .fold(0.0, |i, x| if *x < i { *x } else { i });
        // info!("y axis would be [{}, {}]", min, max);
        let scale: f32 = f32::max(max, min.abs());
        // info!("scale factor is {}", scale);
        let mut scaled_samples: Vec<f32> = one_sixty_ms_downsampled_audio.slice.iter().map(|x| *x / scale ).collect();
        // info!("scaled_samples size is {}", scaled_samples.len());


        // forward transform the signal
        let result = self.r2c.process(&mut scaled_samples, &mut self.spectrum.borrow_mut());

        if result.is_ok() {
            let norm: f32 = (1.0 /(self.spectrum.borrow().len() as f32)).sqrt();
            let norm_2 = norm * norm;
            
            // Normalise im and re by 1 / size, then transform to a vector of the magnitude
            let magnitude: Vec<f32> = self.spectrum.borrow().iter().map(|x| {
                (norm_2 * x.re * x.re + norm_2 * x.im * x.im).sqrt()
            }).collect();

            let out = ObservableFrequencySlice {reals: magnitude};
            self.observers.notify_observers(&out);
        }
    }
}

#[cfg(test)]
#[path = "./fft_spec.rs"]
mod fft_spec;
