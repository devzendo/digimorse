extern crate hamcrest2;

#[cfg(test)]
mod bitvec_source_encoding_builder_spec {
    use rstest::*;
    use std::env;
    use crate::libs::source_encoder::bitvec_source_encoding_builder::BitvecSourceEncodingBuilder;
    use crate::libs::source_encoder::source_encoding::SourceEncodingBuilder;

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
    pub fn empty_storage(fixture: BitvecSourceEncodingBuilderFixture) {
        assert_eq!(fixture.storage.size(), 0);
    }

}
