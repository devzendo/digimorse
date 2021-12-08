use bitvec::prelude::*;
use crate::libs::source_codec::source_encoding::{SOURCE_ENCODER_BLOCK_SIZE_IN_BITS, SourceEncodingExtractor};

/// A SourceDecodingExtractor using the bitvec crate.
pub struct BitvecSourceEncodingExtractor {
    bits: BitVec<Msb0, u8>,
}

impl BitvecSourceEncodingExtractor {
    pub fn new(source: Vec<u8>) -> Self {
        if source.len() != (SOURCE_ENCODER_BLOCK_SIZE_IN_BITS >> 3) {
            panic!("Extractor will not extract from incorrect block sized data ({} bits)", source.len() << 3);
        }
        let bit_vec = BitVec::<Msb0, u8>::from_vec(source);
        Self {
            bits: bit_vec,
        }
    }
    fn drain(&mut self, num_bits: usize) -> BitVec {
        let drained = self.bits.drain(0 .. num_bits);
        let extracted: BitVec = drained.collect();
        return extracted;
    }

    fn extract_n_bits<D: Copy + BitStore>(&mut self, num_bits: usize, max_bits: usize) -> D {
        if num_bits > max_bits {
            panic!("Cannot extract more than 8 bits with extract_8_bits");
        }
        let remaining = self.bits.len();
        if remaining < num_bits {
            panic!("Cannot extract {} bits; {} bits remain", num_bits, remaining);
        }
        let mut extracted = self.drain(num_bits);

        // Perhaps there's a quicker way to fill the vector with 8-num_bits 0's?
        let mut out_bitvec = BitVec::<Msb0, D>::new();
        for _ in 0 .. (max_bits - num_bits) {
            out_bitvec.push(false);
        }
        out_bitvec.append(&mut extracted);

        out_bitvec.as_raw_slice()[0]
    }
}

impl SourceEncodingExtractor for BitvecSourceEncodingExtractor {
    fn remaining(&self) -> usize {
        self.bits.len()
    }

    fn extract_bool(&mut self) -> bool {
        let remaining = self.bits.len();
        if remaining == 0 {
            panic!("Cannot extract 1 bit; {} bits remain", remaining);
        }
        return self.bits.remove(0);
    }

    fn extract_8_bits(&mut self, num_bits: usize) -> u8 {
        // Code works without this optimisation, but can't see how to move it into the generic
        // extract_n_bits function due to inability to express 0 as a generic unsigned type. ?!?!
        if num_bits == 0 {
            return 0;
        }
        return self.extract_n_bits::<u8>(num_bits, 8);
    }

    fn extract_16_bits(&mut self, num_bits: usize) -> u16 {
        // Code works without this optimisation, but can't see how to move it into the generic
        // extract_n_bits function due to inability to express 0 as a generic unsigned type. ?!?!
        if num_bits == 0 {
            return 0;
        }
        return self.extract_n_bits::<u16>(num_bits, 16);
    }

    fn extract_32_bits(&mut self, num_bits: usize) -> u32 {
        // Code works without this optimisation, but can't see how to move it into the generic
        // extract_n_bits function due to inability to express 0 as a generic unsigned type. ?!?!
        if num_bits == 0 {
            return 0;
        }
        return self.extract_n_bits::<u32>(num_bits, 32);
    }
}

#[cfg(test)]
#[path = "./bitvec_source_encoding_extractor_spec.rs"]
mod bitvec_source_encoding_extractor_spec;

