use std::sync::{RwLock, Arc};
use by_address::ByAddress;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

pub trait Observable: Clone + Send {
}

pub trait Observer<O: Observable> {
    fn on_notify(&self, observable: &O);
}

pub trait ObserverList<O: Observable> {
    fn notify_observers(&self, observable: &O);
    fn register_observer(&mut self, observer: Arc<dyn Observer<O>>) -> u64;
    fn unregister_observer(&mut self, observer_hash_to_unregister: u64);
}

pub struct ConcreteObserverList<O: Observable> {
    observers: RwLock<Vec<Arc<dyn Observer<O>>>>,
}

impl<O: Observable> ConcreteObserverList<O> {
    fn new() -> ConcreteObserverList<O> {
        ConcreteObserverList {
            observers: RwLock::new(Vec::new()),
        }
    }

    fn observer_hash(&self, observer: &ByAddress<Arc<dyn Observer<O>>>) -> u64 {
        let mut hasher = DefaultHasher::new();
        observer.hash(&mut hasher);
        hasher.finish()
    }
}

impl<O: Observable> ObserverList<O> for ConcreteObserverList<O> {
    fn notify_observers(&self, observable: &O) {
        for observer in self.observers.read().unwrap().iter() {
            observer.on_notify(observable);
        }
    }

    fn register_observer(&mut self, observer: Arc<dyn Observer<O>>) -> u64 {
        let mut observers = self.observers.write().unwrap();
        //let address = ByAddress(observer);
        observers.push(observer);
        //self.observer_hash(&address)
        (observers.len() - 1) as u64
    }

    fn unregister_observer(&mut self, observer_hash_to_unregister: u64) {
        // self.observers.write().unwrap().retain(|o|
        //                                            {
        //                                                let o_hash = self.observer_hash(o);
        //                                                o_hash != observer_hash_to_unregister
        //                                            } );
        self.observers.write().unwrap().remove(observer_hash_to_unregister as usize);
    }
}


#[cfg(test)]
#[path = "./observer_spec.rs"]
mod observer_spec;
