use std::sync::Arc;
use log::debug;
use crate::libs::patterns::observer::{ConcreteObserverList, Observable, Observer, ObserverList};

pub const OBSERVABLE_BUFFER_SLICE_SIZE: usize = 1920;

const OBSERVABLE_BUFFER_SIZE: usize = OBSERVABLE_BUFFER_SLICE_SIZE * 2;

#[derive(Clone)]
struct ObservableBufferSlice {
    slice: Vec<f32>,
}
impl Observable for ObservableBufferSlice {
}


struct ObservableBuffer {
    buffer: Vec<f32>,
    observers: ConcreteObserverList<ObservableBufferSlice>,
    from: usize,
    to: usize,
}

impl ObservableBuffer {
    pub fn new() -> Self {
        let mut vec = Vec::with_capacity(OBSERVABLE_BUFFER_SIZE);
        vec.resize(OBSERVABLE_BUFFER_SIZE, 0_f32);

        let obs = ConcreteObserverList::new();

        Self {
            buffer: vec,
            observers: obs,
            from: 0,
            to: 0,
        }
    }

    pub fn add_observer(&mut self, observer: Arc<dyn Observer<ObservableBufferSlice>>) {
        self.observers.register_observer(observer);
    }

    pub fn add_sample(&mut self, sample: f32) {
        // TODO cyclic buffer wraparound
        self.buffer[self.to] = sample;
        self.to += 1;
        let stored_amount = self.to - self.from;
        debug!("sample {}; stored amount {}", sample, stored_amount);
        if stored_amount == OBSERVABLE_BUFFER_SLICE_SIZE {
            debug!("notifying observers");
            let slice = &self.buffer[self.from..self.to];
            let observable = ObservableBufferSlice { slice: slice.to_vec() };
            self.observers.notify_observers(&observable);
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
