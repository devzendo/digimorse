use std::mem;
use bitvec::prelude::*;
use log::debug;
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
    fn drain(&mut self, num_bits: usize) -> BitVec {
        let drained = self.bits.drain(0 .. num_bits);
        let mut extracted: BitVec = drained.collect();
        return extracted;
    }
}

impl SourceEncodingExtractor for BitvecSourceEncodingExtractor {
    fn remaining(&self) -> usize {
        self.bits.len()
    }

    fn extract_8_bits(&mut self, num_bits: usize) -> u8 {
        // Code works without this optimisation
        if num_bits == 0 {
            return 0;
        }
        if num_bits > 8 {
            panic!("Cannot extract more than 8 bits with extract_8_bits");
        }
        let remaining = self.bits.len();
        if remaining < num_bits {
            panic!("Cannot extract {} bits; {} bits remain", num_bits, remaining);
        }
        let mut extracted = self.drain(num_bits);
        // debug!("extracted {:#010b}", extracted);

        // Perhaps there's a quicker way to fill the vector with 8-num_bits 0's?
        let mut out_bitvec = BitVec::<Msb0, u8>::new();
        for _ in 0 .. (8 - num_bits) {
            out_bitvec.push(false);
        }
        out_bitvec.append(&mut extracted);

        // debug!("out_bitvec len {} {:#010b}", out_bitvec.len(), out_bitvec);
        out_bitvec.as_raw_slice()[0]
    }

    fn extract_16_bits(&mut self, num_bits: usize) -> u16 {
        // Code works without this optimisation
        if num_bits == 0 {
            return 0;
        }
        if num_bits > 16 {
            panic!("Cannot extract more than 16 bits with extract_16_bits");
        }
        let remaining = self.bits.len();
        if remaining < num_bits {
            panic!("Cannot extract {} bits; {} bits remain", num_bits, remaining);
        }
        let mut extracted = self.drain(num_bits);
        // debug!("extracted {:#018b}", extracted);

        // Perhaps there's a quicker way to fill the vector with 16-num_bits 0's?
        let mut out_bitvec = BitVec::<Msb0, u16>::new();
        for _ in 0 .. (16 - num_bits) {
            out_bitvec.push(false);
        }
        out_bitvec.append(&mut extracted);

        // debug!("out_bitvec len {} {:#016b}", out_bitvec.len(), out_bitvec);
        out_bitvec.as_raw_slice()[0]
    }

    fn extract_32_bits(&mut self, num_bits: usize) -> u32 {
        // Code works without this optimisation
        if num_bits == 0 {
            return 0;
        }
        if num_bits > 32 {
            panic!("Cannot extract more than 32 bits with extract_32_bits");
        }
        let remaining = self.bits.len();
        if remaining < num_bits {
            panic!("Cannot extract {} bits; {} bits remain", num_bits, remaining);
        }
        let mut extracted = self.drain(num_bits);
        // debug!("extracted {:#034b}", extracted);

        // Perhaps there's a quicker way to fill the vector with 32-num_bits 0's?
        let mut out_bitvec = BitVec::<Msb0, u32>::new();
        for _ in 0 .. (32 - num_bits) {
            out_bitvec.push(false);
        }
        out_bitvec.append(&mut extracted);

        // debug!("out_bitvec len {} {:#034b}", out_bitvec.len(), out_bitvec);
        out_bitvec.as_raw_slice()[0]
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

