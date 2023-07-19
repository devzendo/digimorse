use std::fmt::Display;
use std::sync::Arc;
use log::debug;
use crate::libs::patterns::observer::{ConcreteObserverList, Observable, Observer, ObserverList};

pub const OBSERVABLE_BUFFER_SLICE_SIZE: usize = 1920;

const OBSERVABLE_BUFFER_SIZE: usize = OBSERVABLE_BUFFER_SLICE_SIZE * 2;

// T is going to be some primitive type: f32, u32 etc.

#[derive(Clone)]
pub struct ObservableBufferSlice<T> where T: Clone + Copy + Default + Display + Send + Sync {
    pub slice: Vec<T>,
}

impl<T: Clone + Copy + Default + Display + Send + Sync> Observable for ObservableBufferSlice<T> {
}


pub struct ObservableBuffer<T> where T: Clone + Copy + Default + Display + Send + Sync {
    buffer: Vec<T>,
    observers: ConcreteObserverList<ObservableBufferSlice<T>>,
    from: usize,
    to: usize,
}

impl<T: Clone + Copy + Default + Display + Send + Sync> ObservableBuffer<T> {
    pub fn new() -> Self {
        let mut vec = Vec::with_capacity(OBSERVABLE_BUFFER_SIZE);
        vec.resize(OBSERVABLE_BUFFER_SIZE, T::default());

        let obs = ConcreteObserverList::new();

        Self {
            buffer: vec,
            observers: obs,
            from: 0,
            to: 0,
        }
    }

    pub fn add_observer(&mut self, observer: Arc<dyn Observer<ObservableBufferSlice<T>>>) {
        self.observers.register_observer(observer);
    }

    pub fn add_sample(&mut self, sample: T) {
        // TODO cyclic buffer wraparound
        self.buffer[self.to] = sample;
        self.to += 1;
        let stored_amount = self.to - self.from;
        debug!("sample {}; stored amount {}", sample, stored_amount);
        if stored_amount == OBSERVABLE_BUFFER_SLICE_SIZE {
            debug!("notifying observers");
            let slice = &self.buffer[self.from..self.to];
            let observable = ObservableBufferSlice { slice: slice.to_vec() };
            // TODO notification must be done on a separate thread - this is the callback.
            self.observers.notify_observers(&observable);
            self.from += OBSERVABLE_BUFFER_SLICE_SIZE;
        }
    }

    #[cfg(test)]
    fn range(&self) -> (usize, usize) {
        (self.from, self.to)
    }
}


#[cfg(test)]
#[path = "./observable_buffer_spec.rs"]
mod observable_buffer_spec;
