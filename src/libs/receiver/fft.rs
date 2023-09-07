use std::sync::Arc;
use log::debug;
use rustfft::{Fft, FftPlanner};
use rustfft::num_complex::Complex;
// use rustfft::FFTplanner;
// use rustfft::num_complex::Complex;
use crate::libs::buffer_pool::observable_buffer::{OBSERVABLE_BUFFER_SLICE_SIZE, ObservableBufferSlice};
use crate::libs::patterns::observer::{ConcreteObserverList, Observable, Observer, ObserverList};

#[derive(Clone)]
pub struct ObservableFrequencySlice {
    reals: Vec<f32>,
}

impl Observable for ObservableFrequencySlice {
}

pub struct FFTingBufferObserver {
    fft: Arc<dyn Fft<f32>>,
    observers: ConcreteObserverList<ObservableFrequencySlice>,
}

impl FFTingBufferObserver {
    pub fn new() -> Self {
       let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(OBSERVABLE_BUFFER_SLICE_SIZE * 2);

        Self {
            fft,
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
        if slice_len != OBSERVABLE_BUFFER_SLICE_SIZE {
            panic!("Expecting sample buffers of length {} not {}", OBSERVABLE_BUFFER_SLICE_SIZE, slice_len)
        }
        // RefCell for interior mutability since the on_notify method does not have &mut self
        // self.observations.borrow_mut().push(observable.clone());

        // Copy the real data from the one_sixty_ms_downsampled_audio into the first half of the buffer
        // that rustfft uses as input/output - as complex data with a zero imaginary part. // TODO zero pad, fft, emit
        let mut buffer = vec![Complex{ re: 0.0, im: 0.0 }; OBSERVABLE_BUFFER_SLICE_SIZE * 2];
        for i in 0 .. OBSERVABLE_BUFFER_SLICE_SIZE {
            buffer[i] = Complex{ re: one_sixty_ms_downsampled_audio.slice[i], im: 0.0};
        }
        debug!("calling FFT");
        self.fft.process(&mut buffer);
        debug!("called FFT");
        let mut vec = Vec::with_capacity(OBSERVABLE_BUFFER_SLICE_SIZE * 2);
        vec.resize(OBSERVABLE_BUFFER_SLICE_SIZE * 2, 0.0_f32);
        for i in 0 .. OBSERVABLE_BUFFER_SLICE_SIZE * 2 {
            vec[i] = buffer[i].im;
        }
        let out = ObservableFrequencySlice {reals: vec};
        self.observers.notify_observers(&out);
    }
}

#[cfg(test)]
#[path = "./fft_spec.rs"]
mod fft_spec;
