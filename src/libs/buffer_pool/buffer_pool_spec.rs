#[cfg(test)]
mod buffer_pool_spec {
    use std::env;

    use hamcrest2::prelude::*;
    use rstest::*;

    use crate::libs::buffer_pool::buffer_pool::BufferPool;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct BufferPoolFixture {
        buffer_pool: BufferPool,
    }


    #[fixture]
    fn fixture() -> BufferPoolFixture {
        BufferPoolFixture {
            buffer_pool: BufferPool::new(16, 4),
        }
    }

    #[rstest]
    #[serial]
    pub fn allocate_first(mut fixture: BufferPoolFixture) {
        let maybe_allocated = fixture.buffer_pool.allocate();
        assert_that!(&maybe_allocated, some());
        let tuple = maybe_allocated.unwrap();
        assert_that!(tuple.0, equal_to(0));
    }

    #[rstest]
    #[serial]
    pub fn allocate_second(mut fixture: BufferPoolFixture) {
        fixture.buffer_pool.allocate();
        let maybe_allocated = fixture.buffer_pool.allocate();
        assert_that!(&maybe_allocated, some());
        let tuple = maybe_allocated.unwrap();
        assert_that!(tuple.0, equal_to(1));
    }

    #[rstest]
    #[serial]
    pub fn allocate_last(mut fixture: BufferPoolFixture) {
        fixture.buffer_pool.allocate();
        fixture.buffer_pool.allocate();
        fixture.buffer_pool.allocate();
        let maybe_allocated = fixture.buffer_pool.allocate();
        assert_that!(&maybe_allocated, some());
        let tuple = maybe_allocated.unwrap();
        assert_that!(tuple.0, equal_to(3));
    }

    #[rstest]
    #[serial]
    pub fn allocate_exhaust(mut fixture: BufferPoolFixture) {
        fixture.buffer_pool.allocate();
        fixture.buffer_pool.allocate();
        fixture.buffer_pool.allocate();
        fixture.buffer_pool.allocate();
        let maybe_allocated = fixture.buffer_pool.allocate();
        assert_that!(&maybe_allocated, none());
    }

    #[rstest]
    #[serial]
    pub fn free_first(mut fixture: BufferPoolFixture) {
        let maybe_allocated = fixture.buffer_pool.allocate();
        let tuple = maybe_allocated.unwrap();
        assert_eq!(fixture.buffer_pool.free(tuple.0), true);
    }

    #[rstest]
    #[serial]
    pub fn free_first_then_re_allocate(mut fixture: BufferPoolFixture) {
        let maybe_allocated_1 = fixture.buffer_pool.allocate();
        let tuple_1 = maybe_allocated_1.unwrap();
        assert_that!(tuple_1.0, eq(0));
        assert_eq!(fixture.buffer_pool.free(tuple_1.0), true);

        let maybe_allocated_2 = fixture.buffer_pool.allocate();
        let tuple_2 = maybe_allocated_2.unwrap();
        assert_that!(tuple_2.0, eq(0));
        assert_eq!(fixture.buffer_pool.free(tuple_2.0), true);
    }

    #[rstest]
    #[serial]
    pub fn double_free(mut fixture: BufferPoolFixture) {
        let maybe_allocated_1 = fixture.buffer_pool.allocate();
        let tuple_1 = maybe_allocated_1.unwrap();
        assert_eq!(fixture.buffer_pool.free(tuple_1.0), true);

        assert_eq!(fixture.buffer_pool.free(tuple_1.0), false);
    }

    #[rstest]
    #[serial]
    pub fn buffers_are_not_cleared(mut fixture: BufferPoolFixture) {
        let maybe_allocated_1 = fixture.buffer_pool.allocate();
        let tuple_1 = maybe_allocated_1.unwrap();
        {
            let mut guard = tuple_1.1.write().unwrap();
            guard[0] = 3.1415f32;
        }
        fixture.buffer_pool.free(tuple_1.0);

        let maybe_allocated_2 = fixture.buffer_pool.allocate();
        let tuple_2 = maybe_allocated_2.unwrap();
        assert_eq!(tuple_2.1.read().unwrap()[0], 3.1415f32);
    }
}
