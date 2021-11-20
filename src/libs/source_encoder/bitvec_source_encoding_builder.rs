use bitvec::prelude::*;
use crate::libs::source_encoder::source_encoding::{SourceEncodingBuilder, SourceEncoding};

/// A SourceEncodingBuilder using the bitvec crate.
pub struct BitvecSourceEncodingBuilder {
    bits: BitVec,
}

impl BitvecSourceEncodingBuilder {
    pub fn new() -> Self {
        Self {
            bits: BitVec::new(),
        }
    }
}

impl SourceEncodingBuilder for BitvecSourceEncodingBuilder {
    fn size(&self) -> usize {
        0
    }

    fn add_8_bits(&mut self, _data: u8, _num_bits: usize) {
        todo!()
    }

    fn add_16_bits(&mut self, _data: u16, _num_bits: usize) {
        todo!()
    }

    fn add_32_bits(&mut self, _data: u32, _num_bits: usize) {
        todo!()
    }

    fn add_bool(&mut self, _data: bool) {
        todo!()
    }

    fn build(&mut self) -> SourceEncoding {
        todo!()
    }
}


#[cfg(test)]
#[path = "./bitvec_source_encoding_builder_spec.rs"]
mod bitvec_source_encoding_builder_spec;
