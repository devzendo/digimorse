#[cfg(test)]
mod observer_spec {
    use crate::libs::patterns::observer::{ConcreteObserverList, Observable, Observer, ObserverList};
    use std::cell::RefCell;
    use std::sync::Arc;

    #[ctor::ctor]
    fn before_each() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    #[derive(Clone)]
    struct MyObservable {
        age: u8,
        name: String,
    }
    impl Observable for MyObservable {
    }

    struct MyObserver {
        observations: RefCell<Vec<MyObservable>>,
    }
    impl MyObserver {
        fn new() -> MyObserver {
            MyObserver {
                observations: RefCell::new(Vec::new()),
            }
        }
        fn observations(&self) -> Vec<MyObservable> {
            self.observations.borrow().clone()
        }
    }
    impl Observer<MyObservable> for MyObserver {
        fn on_notify(&self, observable: &MyObservable) {
            // RefCell for interior mutability since the on_notify method does not have &mut self
            self.observations.borrow_mut().push(observable.clone());
        }
    }
    #[test]
    fn observe_event() {
        let mut list = ConcreteObserverList::new();
        let observer = MyObserver::new();
        let arc_observer = Arc::new(observer);
        let arc_observer_cloned = arc_observer.clone();
        let observer_id = list.register_observer(arc_observer_cloned);

        list.notify_observers(&MyObservable { age: 52, name: "Matt".to_string() });
        list.notify_observers(&MyObservable { age: 21, name: "Morten".to_string() });

        let arc_observer_cloned_again = arc_observer.clone();

        let observations = arc_observer_cloned_again.observations();
        assert_eq!(observations.len(), 2);
        assert_eq!(observations.get(0).unwrap().age, 52);
        assert_eq!(observations.get(0).unwrap().name, "Matt".to_string());
        assert_eq!(observations.get(1).unwrap().age, 21);
        assert_eq!(observations.get(1).unwrap().name, "Morten".to_string());

        list.unregister_observer(observer_id);
        list.notify_observers(&MyObservable { age: 89, name: "Zeus".to_string() });

        let further_observations = arc_observer_cloned_again.observations();
        assert_eq!(further_observations.len(), 2); // no increase

        // TODO the index of any other observers will be invalid after unregister_observer
    }
}
