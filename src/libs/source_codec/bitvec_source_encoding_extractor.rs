use bitvec::prelude::*;
use crate::libs::source_codec::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, SourceEncodingExtractor};

/// A SourceDecodingExtractor using the bitvec crate.
pub struct BitvecSourceEncodingExtractor {
    bits: BitVec::<Msb0, u8>,
}

impl BitvecSourceEncodingExtractor {
    pub fn new(source: Vec<u8>) -> Self {
        if source.len() != (SOURCE_ENCODER_BLOCK_SIZE_IN_BITS >> 3) {
            panic!("Extractor will not extract from incorrect block sized data ({} bits)", source.len() << 3);
        }
        let mut bit_vec = BitVec::<Msb0, u8>::from_vec(source);
        Self {
            bits: bit_vec,
        }
    }
}

impl SourceEncodingExtractor for BitvecSourceEncodingExtractor {
    fn remaining(&self) -> usize {
        todo!()
    }

    fn extract_8_bits(&mut self, num_bits: usize) -> u8 {
        todo!()
    }

    fn extract_16_bits(&mut self, num_bits: usize) -> u16 {
        todo!()
    }

    fn extract_32_bits(&mut self, num_bits: usize) -> u32 {
        todo!()
    }
}
/*
    let mut bit_vec = BitVec::<Msb0, u8>::from_vec(encoded_block);
    let bit_slice = bit_vec.as_bitslice();
    loop {
        bit_slice.
        unsafe {
            let data_slice = BitSlice::<Msb0, _>::get_unchecked(0);
            let data_sub_slice = data_slice.get_unchecked_mut((max_bits - num_bits)..max_bits);
            let mut data_sub_bit_vec = data_sub_slice.to_bitvec();
            self.bits.append(&mut data_sub_bit_vec);
        }

    }

 */

#[cfg(test)]
#[path = "./bitvec_source_encoding_extractor_spec.rs"]
mod bitvec_source_encoding_extractor_spec;

