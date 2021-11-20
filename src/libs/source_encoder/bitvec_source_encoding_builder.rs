use bitvec::prelude::*;
use crate::libs::source_encoder::source_encoding::{SourceEncodingBuilder, SourceEncoding, SOURCE_ENCODER_BLOCK_SIZE_IN_BITS};

/// A SourceEncodingBuilder using the bitvec crate.
pub struct BitvecSourceEncodingBuilder {
    bits: BitVec::<Msb0, u8>,
    end: bool,
}

impl BitvecSourceEncodingBuilder {
    pub fn new() -> Self {
        let mut bit_vec = BitVec::<Msb0, u8>::with_capacity(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        bit_vec.set_uninitialized(false);
        Self {
            bits: bit_vec,
            end: false,
        }
    }

    fn panic_if_full(&self, num_bits_being_added: usize) {
        if self.size() + num_bits_being_added > SOURCE_ENCODER_BLOCK_SIZE_IN_BITS {
            panic!("Adding {} bit(s) would exhaust storage", num_bits_being_added);
        }
    }
}

impl SourceEncodingBuilder for BitvecSourceEncodingBuilder {
    fn size(&self) -> usize {
        self.bits.len()
    }

    fn remaining(&self) -> usize {
        SOURCE_ENCODER_BLOCK_SIZE_IN_BITS - self.bits.len()
    }

    fn add_8_bits(&mut self, mut data: u8, num_bits: usize) {
        // The code works without this optimisation.
        if num_bits == 0 {
            return;
        }
        if num_bits > 8 {
            panic!("Cannot add more than 8 bits with add_8_bits, was trying to add {}", num_bits);
        }
        self.panic_if_full(num_bits);
        let data_slice = BitSlice::<Msb0, _>::from_element_mut(&mut data);
        unsafe {
            let data_sub_slice = data_slice.get_unchecked_mut((8 - num_bits)..8);
            let mut data_sub_bit_vec = data_sub_slice.to_bitvec();
            self.bits.append(&mut data_sub_bit_vec);
        }
    }

    fn add_16_bits(&mut self, _data: u16, _num_bits: usize) {
        todo!()
    }

    fn add_32_bits(&mut self, _data: u32, _num_bits: usize) {
        todo!()
    }

    fn add_bool(&mut self, data: bool) {
        self.panic_if_full(1);
        self.bits.push(data);
    }

    fn set_end(&mut self) {
        self.end = true;
    }

    fn build(&mut self) -> SourceEncoding {
        // Extend the bitvec to its capacity
        unsafe {
            self.bits.set_len(SOURCE_ENCODER_BLOCK_SIZE_IN_BITS);
        }
        let out = SourceEncoding {
            block: self.bits.as_raw_slice().to_vec(),
            is_end: self.end,
        };
        self.bits.clear();
        self.bits.set_uninitialized(false);
        out
    }
}


#[cfg(test)]
#[path = "./bitvec_source_encoding_builder_spec.rs"]
mod bitvec_source_encoding_builder_spec;
