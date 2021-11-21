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
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_a_boolean(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_bool(true);
        assert_eq!(fixture.storage.size(), 1);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 1);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![128, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_several_booleans(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(false);
        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(true);
        assert_eq!(fixture.storage.size(), 9);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 9);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0b11010011, 0b10000000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn blocks_are_not_end_blocks_by_default(mut fixture: BitvecSourceEncodingBuilderFixture) {
        let encoding = fixture.storage.build();
        assert_eq!(encoding.is_end, false);
    }

    #[rstest]
    pub fn blocks_can_be_set_as_end_blocks(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.set_end();
        let encoding = fixture.storage.build();
        assert_eq!(encoding.is_end, true);
    }

    #[rstest]
    pub fn blocks_end_state_is_reset_on_build(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.set_end();
        fixture.storage.build();
        let encoding = fixture.storage.build();
        assert_eq!(encoding.is_end, false);
    }

    #[rstest]
    pub fn build_clears_for_new_block(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(true);
        let first_encoding = fixture.storage.build();

        let second_encoding = fixture.storage.build();
        let second_vec = second_encoding.block;
        assert_that!(&second_vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(second_vec, vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    // build up some data, build() it, build up some more, check original block to ensure it has
    // not been overwritten
    pub fn building_again_does_not_affect_previously_built(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(true);
        assert_eq!(fixture.storage.size(), 4);
        let encoding = fixture.storage.build();
        let first_vec = encoding.block;
        assert_that!(&first_vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(first_vec, vec![0b10110000, 0, 0, 0, 0, 0, 0, 0]);

        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        assert_eq!(first_vec, vec![0b10110000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    // build up some data, build() it, build up some more, build() it and check it's the 2nd data.
    pub fn each_built_block_is_new_storage(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(true);
        assert_eq!(fixture.storage.size(), 4);
        let first_encoding = fixture.storage.build();
        let first_vec = first_encoding.block;
        assert_that!(&first_vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(first_vec, vec![0b10110000, 0, 0, 0, 0, 0, 0, 0]);

        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        fixture.storage.add_bool(false);
        fixture.storage.add_bool(true);
        let second_encoding = fixture.storage.build();
        let second_vec = second_encoding.block;
        assert_that!(&second_vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(second_vec, vec![0b01010000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    #[should_panic]
    // add more than the block size of data - what should happen? it's up to the caller to check the
    // current size before adding data, so it's probably best to panic.
    // build up some data, build() it, build up some more, build() it and check it's the 2nd data.
    pub fn panics_after_full_adding_bools(mut fixture: BitvecSourceEncodingBuilderFixture) {
        for n in 0..=SOURCE_ENCODER_BLOCK_SIZE_IN_BITS {
            fixture.storage.add_bool(true);
        }
    }

    #[rstest]
    pub fn does_not_panic_at_full_adding_bools(mut fixture: BitvecSourceEncodingBuilderFixture) {
        for n in 0..SOURCE_ENCODER_BLOCK_SIZE_IN_BITS {
            fixture.storage.add_bool(true);
        }
    }



    #[rstest]
    #[should_panic]
    pub fn panics_after_full_adding_u8(mut fixture: BitvecSourceEncodingBuilderFixture) {
        for n in 0..=(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS/8) {
            fixture.storage.add_8_bits(0, 8);
        }
    }

    #[rstest]
    pub fn does_not_panic_at_full_adding_u8(mut fixture: BitvecSourceEncodingBuilderFixture) {
        for n in 0..(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS/8) {
            fixture.storage.add_8_bits(0, 8);
        }
    }

    #[rstest]
    pub fn add_a_full_byte(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_8_bits(0b10000011, 8);
        assert_eq!(fixture.storage.size(), 8);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 8);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0b10000011, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_a_partial_byte(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_8_bits(0b00001101, 4);
        assert_eq!(fixture.storage.size(), 4);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 4);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0b11010000, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_no_bits_from_a_byte(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_8_bits(0b11111111, 0); // pointless, but why not?
        assert_eq!(fixture.storage.size(), 0);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    #[should_panic]
    pub fn add_more_than_8_bits_from_a_byte_panics(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_8_bits(0b11111111, 9);
    }



    #[rstest]
    #[should_panic]
    pub fn panics_after_full_adding_u16(mut fixture: BitvecSourceEncodingBuilderFixture) {
        for n in 0..=(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS/16) {
            fixture.storage.add_16_bits(0, 16);
        }
    }

    #[rstest]
    pub fn does_not_panic_at_full_adding_u16(mut fixture: BitvecSourceEncodingBuilderFixture) {
        for n in 0..(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS/16) {
            fixture.storage.add_16_bits(0, 16);
        }
    }

    #[rstest]
    pub fn add_a_full_word(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_16_bits(0b1000000000000011, 16);
        assert_eq!(fixture.storage.size(), 16);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 16);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0b10000000, 0b00000011, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_a_partial_word(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_16_bits(0b1110001101, 10);
        assert_eq!(fixture.storage.size(), 10);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 10);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0b11100011, 0b01000000, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_no_bits_from_a_word(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_16_bits(0b1111111111111111, 0); // pointless, but why not?
        assert_eq!(fixture.storage.size(), 0);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    #[should_panic]
    pub fn add_more_than_16_bits_from_a_word_panics(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_16_bits(0b1111000000001111, 17);
    }


    #[rstest]
    #[should_panic]
    pub fn panics_after_full_adding_u32(mut fixture: BitvecSourceEncodingBuilderFixture) {
        for n in 0..=(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS/32) {
            fixture.storage.add_32_bits(0, 32);
        }
    }

    #[rstest]
    pub fn does_not_panic_at_full_adding_u32(mut fixture: BitvecSourceEncodingBuilderFixture) {
        for n in 0..(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS/32) {
            fixture.storage.add_32_bits(0, 32);
        }
    }

    #[rstest]
    pub fn add_a_full_dword(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_32_bits(0b10000000000000000000000000000011, 32);
        assert_eq!(fixture.storage.size(), 32);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 32);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0b10000000, 0b00000000, 0b00000000, 0b00000011, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_a_partial_dword(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_32_bits(0b00000000000010000000001110001101, 20);
        assert_eq!(fixture.storage.size(), 20);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 20);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0b10000000, 0b00111000, 0b11010000, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    pub fn add_no_bits_from_a_dword(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_32_bits(0b11111110000000000000000111111111, 0); // pointless, but why not?
        assert_eq!(fixture.storage.size(), 0);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));
        assert_eq!(vec, vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[rstest]
    #[should_panic]
    pub fn add_more_than_32_bits_from_a_dword_panics(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_32_bits(0b11110000000000000000000000001111, 33);
    }






    #[rstest]
    pub fn add_a_mix_of_types_and_partials(mut fixture: BitvecSourceEncodingBuilderFixture) {
        fixture.storage.add_bool(false);
        fixture.storage.add_8_bits(0b00000101, 3);
        fixture.storage.add_bool(false);
        fixture.storage.add_8_bits(0b10000111, 8);
        fixture.storage.add_16_bits(0b00010111, 5);
        fixture.storage.add_bool(true);
        fixture.storage.add_16_bits(0b1010101000010111, 16);
        fixture.storage.add_bool(true);
        fixture.storage.add_32_bits(0b1010101000010111, 3);
        assert_eq!(fixture.storage.size(), 39);
        assert_eq!(fixture.storage.remaining(), SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - 39);
        let encoding = fixture.storage.build();
        let vec = encoding.block;
        assert_that!(&vec, len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS / 8));           //  v
        assert_eq!(vec, vec![0b01010100, 0b00111101, 0b11110101, 0b01000010, 0b11111110, 0, 0, 0]);
    }
}
