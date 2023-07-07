#[cfg(test)]
mod observable_buffer_spec {
    use std::cell::RefCell;
    use std::env;
    use std::sync::{Arc, Mutex};

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

    pub struct ObservableBufferFixture {
        observable_buffer: ObservableBuffer,
        observations: RefCell<Vec<ObservableBufferSlice>>,
    }

    impl ObservableBufferFixture {
        fn observations(&self) -> Vec<ObservableBufferSlice> {
            self.observations.borrow().clone()
        }
    }

    impl Observer<ObservableBufferSlice> for ObservableBufferFixture {
        fn on_notify(&self, observable: &ObservableBufferSlice) {
            // RefCell for interior mutability since the on_notify method does not have &mut self
            self.observations.borrow_mut().push(observable.clone());
        }
    }


    #[fixture]
    fn fixture() -> Arc<Mutex<ObservableBufferFixture>> {
        let mut fixture = ObservableBufferFixture {
            observable_buffer: ObservableBuffer::new(),
            observations: RefCell::new(vec![]),
        };
        let mut arc_fixture = Arc::new(fixture);
        arc_fixture.observable_buffer.add_observer(arc_fixture.clone());
        arc_fixture
    }

    #[rstest]
    #[serial]
    pub fn initial_state(mut fixture: Arc<ObservableBufferFixture>) {
        assert_that!(fixture.observations.borrow().is_empty(), equal_to(true));
        assert_that!(fixture.observable_buffer.range(), equal_to((0, 0)));
    }

    #[rstest]
    #[serial]
    pub fn add_first_sample(mut fixture: Arc<ObservableBufferFixture>) {
        fixture.observable_buffer.add_sample(0.0);
        assert_that!(fixture.observable_buffer.range(), equal_to((0, 1)));
    }

    #[rstest]
    #[serial]
    pub fn emit_first_slice(mut fixture: Arc<ObservableBufferFixture>) {
        for i in 0..OBSERVABLE_BUFFER_SLICE_SIZE {
            fixture.observable_buffer.add_sample((i as f32).sin());
        }
        assert_that!(fixture.observable_buffer.range(), equal_to((0, OBSERVABLE_BUFFER_SLICE_SIZE)));
        assert_that!(fixture.observations.borrow().len(), equal_to(1));
    }
}
