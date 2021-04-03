use std::sync::{RwLock, Arc};

pub trait Observable: Clone + Send {
}

pub trait Observer<O: Observable> {
    fn on_notify(&self, observable: &O);
}

pub trait ObserverList<O: Observable> {
    fn notify_observers(&self, observable: &O);
    fn register_observer(&mut self, observer: Arc<dyn Observer<O>>) -> usize;
    fn unregister_observer(&mut self, observer_id_to_unregister: usize);
}

pub struct ConcreteObserverList<O: Observable> {
    observers: RwLock<Vec<(bool, Arc<dyn Observer<O>>)>>,
}

impl<O: Observable> ConcreteObserverList<O> {
    pub(crate) fn new() -> ConcreteObserverList<O> {
        ConcreteObserverList {
            observers: RwLock::new(Vec::new()),
        }
    }
}

impl<O: Observable> ObserverList<O> for ConcreteObserverList<O> {
    fn notify_observers(&self, observable: &O) {
        for observer in self.observers.read().unwrap().iter() {
            if observer.0 {
                observer.1.on_notify(observable);
            }
        }
    }

    fn register_observer(&mut self, observer: Arc<dyn Observer<O>>) -> usize {
        let mut observers = self.observers.write().unwrap();
        observers.push((true, observer));
        observers.len() - 1
    }

    fn unregister_observer(&mut self, observer_id_to_unregister: usize) {
        self.observers.write().unwrap()[observer_id_to_unregister].0 = false;
    }
}


#[cfg(test)]
#[path = "./observer_spec.rs"]
mod observer_spec;
