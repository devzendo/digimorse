extern crate hamcrest2;

#[cfg(test)]
mod bitvec_source_encoding_builder_spec {
    use rstest::*;
    use hamcrest2::prelude::*;
    use std::env;
    use crate::libs::source_encoder::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
    use crate::libs::source_encoder::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, SourceEncodingBuilder};

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    pub struct BitvecSourceEncodingBuilderFixture {
        storage: Box<dyn SourceEncodingBuilder>,
    }

    #[fixture]
    fn fixture() -> BitvecSourceEncodingBuilderFixture {
        BitvecSourceEncodingBuilderFixture {
            storage: Box::new(BitvecSourceEncodingBuilder::new())
        }
    }

    #[rstest]
    pub fn empty_storage(mut fixture: BitvecSourceEncodingBuilderFixture) {
        assert_eq!(fixture.storage.size(), 0);
        let encoding = fixture.storage.build();
        assert_eq!(encoding.is_end, false);
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_a_boolean(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_bool(true);
        assert_eq!(fixture.storage.size(), 1);
        let encoding = fixture.storage.build();
        assert_eq!(encoding.is_end, false);
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![128, 0, 0, 0, 0, 0, 0, 0]);
    }

    // TODO
    // build up some data, build() it, build up some more, check original block to ensure it has
    // not been overwritten

    // build up some data, build() it, build up some more, build() it and check it's the 2nd data.

    // add more than the block size of data - what should happen? it's up to the caller to check the
    // current size before adding data, so it's probably best to panic.
}
