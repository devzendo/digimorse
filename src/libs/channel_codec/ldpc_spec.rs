extern crate hamcrest2;

#[cfg(test)]
mod ldpc_spec {
    use std::env;
    use hamcrest2::prelude::*;
    use log::debug;
    use pretty_hex::*;
    use crate::libs::util::util::vec_to_array;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

}
