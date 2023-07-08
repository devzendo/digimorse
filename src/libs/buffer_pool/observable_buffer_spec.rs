#[cfg(test)]
mod observable_buffer_spec {
    use std::cell::RefCell;
    use std::env;
    use std::sync::Arc;

    use hamcrest2::prelude::*;
    use rstest::*;
    use crate::libs::buffer_pool::observable_buffer::{OBSERVABLE_BUFFER_SLICE_SIZE, ObservableBuffer, ObservableBufferSlice};
    use crate::libs::patterns::observer::Observer;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}


    struct ObservableBufferObserver {
        observations: RefCell<Vec<ObservableBufferSlice<u32>>>,
    }

    impl ObservableBufferObserver {
        fn observations(&self) -> Vec<ObservableBufferSlice<u32>> {
            self.observations.borrow().clone()
        }
    }

    impl Observer<ObservableBufferSlice<u32>> for ObservableBufferObserver {
        fn on_notify(&self, observable: &ObservableBufferSlice<u32>) {
            // RefCell for interior mutability since the on_notify method does not have &mut self
            self.observations.borrow_mut().push(observable.clone());
        }
    }


    pub struct ObservableBufferFixture {
        observable_buffer: ObservableBuffer<u32>,
        observer: Arc<ObservableBufferObserver>,
        sample_count: u32,
    }

    impl ObservableBufferFixture {
        fn add_sample(&mut self) {
            self.observable_buffer.add_sample(self.sample_count);
            self.sample_count += 1;
        }

        fn add_slice_of_samples(&mut self) {
            for i in 0..OBSERVABLE_BUFFER_SLICE_SIZE {
                self.add_sample();
            }
        }
    }

    #[fixture]
    fn fixture() -> ObservableBufferFixture {
        let observer = Arc::new(ObservableBufferObserver { observations: RefCell::new(vec![]) });

        let mut fixture = ObservableBufferFixture {
            observable_buffer: ObservableBuffer::new(),
            observer: observer.clone(),
            sample_count: 0,
        };
        fixture.observable_buffer.add_observer(observer);
        fixture
    }

    #[rstest]
    #[serial]
    pub fn initial_state(fixture: ObservableBufferFixture) {
        assert_that!(fixture.observer.observations().is_empty(), equal_to(true));
        assert_that!(fixture.observable_buffer.range(), equal_to((0, 0)));
    }

    #[rstest]
    #[serial]
    pub fn add_first_sample(mut fixture: ObservableBufferFixture) {
        fixture.observable_buffer.add_sample(0);
        assert_that!(fixture.observable_buffer.range(), equal_to((0, 1)));
    }

    #[rstest]
    #[serial]
    pub fn emit_first_slice(mut fixture: ObservableBufferFixture) {
        fixture.add_slice_of_samples();
        assert_that!(fixture.observable_buffer.range(), equal_to((0, OBSERVABLE_BUFFER_SLICE_SIZE)));
        let observations = fixture.observer.observations();
        assert_that!(observations.len(), equal_to(1));
        let observation = observations.get(0).unwrap();
        for i in 0..OBSERVABLE_BUFFER_SLICE_SIZE {
            assert_that!(observation.slice[i], equal_to(i as u32));
        }
    }
}
