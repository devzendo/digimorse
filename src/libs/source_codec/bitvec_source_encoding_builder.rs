use bitvec::prelude::*;
use crate::libs::source_codec::source_encoding::{SourceEncodingBuilder, SourceEncoding};

/// A SourceEncodingBuilder using the bitvec crate.
pub struct BitvecSourceEncodingBuilder {
    bits: BitVec::<Msb0, u8>,
    end: bool,
    block_size_in_bits: usize,
}

impl BitvecSourceEncodingBuilder {
    pub fn new(block_size_in_bits: usize) -> Self {
        if block_size_in_bits == 0 || block_size_in_bits & 0x07 != 0 {
            panic!("Source encoding builder block size must be a multiple of 8 bits");
        }

        let mut bit_vec = BitVec::<Msb0, u8>::with_capacity(block_size_in_bits);
        bit_vec.set_uninitialized(false);
        Self {
            bits: bit_vec,
            end: false,
            block_size_in_bits
        }
    }

    fn panic_if_full(&self, num_bits_being_added: usize) {
        if self.size() + num_bits_being_added > self.block_size_in_bits {
            panic!("Adding {} bit(s) would exhaust storage", num_bits_being_added);
        }
    }

    fn pack_data_bits<D: BitStore>(&mut self, data: &mut D, num_bits: usize, max_bits: usize) {
        // The code works without this optimisation.
        if num_bits == 0 {
            return;
        }
        self.panic_if_full(num_bits);
        let data_slice = BitSlice::<Msb0, _>::from_element_mut(data);
        unsafe {
            let data_sub_slice = data_slice.get_unchecked_mut((max_bits - num_bits)..max_bits);
            let mut data_sub_bit_vec = data_sub_slice.to_bitvec();
            self.bits.append(&mut data_sub_bit_vec);
        }
    }
}

impl SourceEncodingBuilder for BitvecSourceEncodingBuilder {
    fn size(&self) -> usize {
        self.bits.len()
    }

    fn remaining(&self) -> usize {
        self.block_size_in_bits - self.bits.len()
    }

    fn add_8_bits(&mut self, mut data: u8, num_bits: usize) {
        if num_bits > 8 {
            panic!("Cannot add more than 8 bits with add_8_bits, was trying to add {}", num_bits);
        }
        self.pack_data_bits::<u8>(&mut data, num_bits, 8);
    }

    fn add_16_bits(&mut self, mut data: u16, num_bits: usize) {
        if num_bits > 16 {
            panic!("Cannot add more than 16 bits with add_16_bits, was trying to add {}", num_bits);
        }
        self.pack_data_bits::<u16>(&mut data, num_bits, 16);
    }

    fn add_32_bits(&mut self, mut data: u32, num_bits: usize) {
        if num_bits > 32 {
            panic!("Cannot add more than 32 bits with add_32_bits, was trying to add {}", num_bits);
        }
        self.pack_data_bits::<u32>(&mut data, num_bits, 32);
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
            self.bits.set_len(self.block_size_in_bits);
        }
        let out = SourceEncoding {
            block: self.bits.as_raw_slice().to_vec(),
            is_end: self.end,
        };
        self.bits.clear();
        self.bits.set_uninitialized(false);
        self.end = false;
        out
    }
}


#[cfg(test)]
#[path = "./bitvec_source_encoding_builder_spec.rs"]
mod bitvec_source_encoding_builder_spec;
