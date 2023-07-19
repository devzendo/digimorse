use std::sync::Arc;

use crate::libs::buffer_pool::observable_buffer::{OBSERVABLE_BUFFER_SLICE_SIZE, ObservableBufferSlice};
use crate::libs::patterns::observer::{ConcreteObserverList, Observable, Observer, ObserverList};

#[derive(Clone)]
pub struct ObservableFrequencySlice {
    // TODO output from FFT, whatever that is.
}

impl Observable for ObservableFrequencySlice {
}

pub struct FFTingBufferObserver {
    observers: ConcreteObserverList<ObservableFrequencySlice>,
}

impl FFTingBufferObserver {
    pub fn new() -> Self {
        Self {
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
        // TODO zero pad, fft, emit
        let out = ObservableFrequencySlice {};
        self.observers.notify_observers(&out);
    }
}

#[cfg(test)]
#[path = "./fft_spec.rs"]
mod fft_spec;
