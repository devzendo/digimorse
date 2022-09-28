// A (252,126) Low-Density Parity-Check code, giving 126 bits of redundant parity information.
// Characteristics required: (From section 3.4 of "Iterative Error Correction", Prof. Sarah J.
// Johnson. All page references are to this book.)
// * At least a girth of 6: there should be no 4-cycles in the Tanner graph.
// * Wc (column weight) of 3 (see p. 77)
// * Wr (row weight) of 6: (set by generator software: a doubling of Wc)
// * Regular (irregular could have an improved threshold performance; however the library I'm using
//   only supports regular).
// * Randomly-allocated: "in many cases, randomly allocating the entries in H will produce a
//   reasonable LDPC code" (p75)
// Parity-check matrix: M x N where
// M = #rows = the redundancy or number of parity check constraints (126)
// N = #columns = number of codeword bits (252)
// Generator matrix: N x K where
// N = #rows = number of codeword bits (252)
// K = #columns = number of message bits (126 = 112 encoder bits + 14 CRC bits)
//
// 1) Generate a Mackay-Neal constructed parity-check matrix:
//    Using Radford M. Neal's LDPC-codes with the following command:
//    make-ldpc parity_check_matrix.pchk 126 252 22020 evenboth 3 no4cycle
//                               rows ___/    \    \____ seed
//                                             \___ cols
//    (The 22020 here is a random seed)
// 2) Convert this to alist format file with the command:
//    Using LDPC-codes:
//    pchk-to-alist -z parity_check_matrix.pchk parity_check_matrix.alist
//    Note: alist format has rows columns as its first line. So this should be 126 252
// 3) Convert the .pchk to a (dense) text format file with the LDPC-codes command:
//    print-pchk -d parity_check_matrix.pchk > parity_check_matrix.txt
// 4) The alist file is read and converted into generated Rust code in parity_check_matrix.rs
//    The code to do this conversion is the (ignored, manually invoked) test code in ldpc_spec.rs,
//    test generate_rust_for_parity_check_matrix().
//
// Unknowns:
// a) Why, when the above has generated Rust, and this is used to create a LinearCode, does the
//    generator matrix have the dimensions swapped?
//
// TODO generate many matrixes and evaluate their error correction performance

use std::fmt;

use ldpc::codes::LinearCode;
use log::{debug, info};
use metered::time_source::{Instant, StdInstant};
use sparse_bin_mat::{BinNum, SparseBinMat, SparseBinVec, SparseBinVecBase};

use crate::libs::channel_codec::crc::CRC;
use crate::libs::sparse_binary_matrix::ColumnAccess;

use super::parity_check_matrix::LDPC;

// Just to start the lazy_static, and log how long it takes to initialise.
pub fn init_ldpc() {
    let ldpc_init_duration = StdInstant::now();
    info!("LDPC-parity ({}, {})", LDPC.parity_check_matrix().number_of_rows(), LDPC.parity_check_matrix().number_of_columns());
    info!("LDPC-generator ({}, {})", LDPC.generator_matrix().number_of_rows(), LDPC.generator_matrix().number_of_columns());
    info!("LDPC initialised in {}ms", ldpc_init_duration.elapsed_time());
}

pub struct SparseBinVecAppender {
    capacity: usize,
    positions: Vec<usize>,
    curr_position: usize,
    debug: bool,
}

impl SparseBinVecAppender {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            positions: Vec::new(),
            curr_position: 0,
            debug: false,
        }
    }

    pub fn debug(&mut self) {
        self.debug = !self.debug;
    }

    pub fn to_sparse_bin_vec(&self) -> SparseBinVec {
        SparseBinVec::new(self.capacity, self.positions.clone())
    }

    pub fn append_bit(&mut self, bit: u8) {
        if self.debug {
            debug!("Appending [{}]={}", self.curr_position, bit);
        }
        if self.curr_position == self.capacity {
            panic!("SparseBinVecAppender append_bit would overrun the vector")
        }
        if bit == 0x01 {
            self.positions.push(self.curr_position);
        }
        self.curr_position += 1;
    }

    pub fn append_u8(&mut self, byte: u8) {
        let mut store = byte;
        for _ in 0..8 {
            self.append_bit(if store & 0x80 == 0x80 { 1 } else { 0 });
            store <<= 1;
        }
    }

    pub fn append_u8s(&mut self, bytes: &[u8]) {
        for byte in bytes.iter() {
            self.append_u8(*byte);
        }
    }

    pub fn append_crc(&mut self, word: CRC) {
        let mut store = word << 2; // skip 2 most significant bits
        for _ in 0..14 {
            self.append_bit(if store & 0x8000 == 0x8000 { 1 } else { 0 });
            store <<= 1;
        }
    }
}

pub fn encode_message_to_sparsebinvec(source_encoding: &[u8], crc: CRC) -> SparseBinVec {
    // Serialise source_encoding + crc into a SparseBinVec
    let mut appender = SparseBinVecAppender::new(126);
    // appender.debug();
    //debug!("Appending source encoding");
    appender.append_u8s(source_encoding);
    //debug!("Appending crc");
    // appender.debug();
    appender.append_crc(crc);
    //debug!("Converting to sparse_bin_vec");
    appender.to_sparse_bin_vec()
}

// A copy of Maxime Tremblay's ldpc FlipDecoder, but directly using our LDPC static LinearCode.
#[derive(Debug, Clone)]
pub struct LocalFlipDecoder {
}

impl LocalFlipDecoder {
    pub fn new() -> Self {
        Self { }
    }
}

impl LocalFlipDecoder
{
    pub fn decode<T>(&self, message: &SparseBinVecBase<T>) -> SparseBinVec
        where
            T: std::ops::Deref<Target = [usize]>,
    {
        let mut syndrome = LDPC.syndrome_of(message);
        let mut output = SparseBinVec::new(message.len(), message.as_slice().to_vec());
        while let Some(bit) = self.find_flippable(&syndrome) {
            let update = SparseBinVec::new(LDPC.len(), vec![bit]);
            syndrome = &syndrome + &LDPC.syndrome_of(&update);
            output = &output + &update;
        }
        output
    }

    fn find_flippable(&self, syndrome: &SparseBinVec) -> Option<usize> {
        LDPC.bit_adjacencies().rows().position(|checks| {
            let number_unsatisfied = checks
                .non_trivial_positions()
                .filter(|check| syndrome.is_one_at(*check).unwrap_or(false))
                .count();
            number_unsatisfied > checks.weight() / 2
        })
    }
}

impl fmt::Display for LocalFlipDecoder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Local Flip decoder")
    }
}


// An implementation the bit-flipping decoder from "Iterative Error Detection", p56.
// Note that indices in the book start at 1; in this code they start at 0.
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct JohnsonFlipDecoder {
    Imax: usize,
}

#[allow(non_snake_case)]
impl JohnsonFlipDecoder {
    pub fn new(Imax: usize) -> Self {
        Self { Imax }
    }
}

#[allow(non_snake_case)]
impl JohnsonFlipDecoder
{
    pub fn decode<T>(&self, y: &SparseBinVecBase<T>, code: &LinearCode) -> SparseBinVec
        where
            T: std::ops::Deref<Target=[usize]>,
    {
        let m = code.parity_check_matrix().number_of_rows();
        let N = y.len(); // message length
        debug!("m={}, N={}", m, N);
        // Initialisation
        let mut Mi = SparseBinVec::new(y.len(), y.as_slice().to_vec());
        let mut iteration = 0;

        // Iterate
        loop {
            debug!("iteration {}", iteration);

            // Step 1: Check messages
            debug!("Step 1: Check messages");
            let mut Eji = SparseBinMat::zeros(m, N);
            for j in 0..m {
                let Bj = code.parity_check_matrix().row(j).unwrap();
                let Bj_positions = Bj.non_trivial_positions().into_iter().collect::<Vec<usize>>();
                debug!("B_{}={}", j, Bj);
                for i in Bj_positions.iter() {
                    let i_primes = Bj_positions.clone().into_iter()
                        .filter(|x| *x != *i)
                        .collect::<Vec<usize>>();
                    let sigma = i_primes.iter().fold(BinNum::zero(), |sum, Mi_prime| sum + Mi.get(*Mi_prime).unwrap());
                    debug!("i={}, i'={:?}, E{},{}=={}", i, i_primes, j, i, sigma);
                    Eji = Eji.emplace_at(sigma, j, *i);
                }
            }

            // Step 2: Bit messages
            debug!("Step 2: Bit messages");
            for i in 0..N {
                let Ai = code.parity_check_matrix().column(i).unwrap();
                let yi = y.get(i).unwrap().is_one();
                debug!("i={}, yi={}, Ai={} Ai.len={}", i, yi, Ai, Ai.weight());
                // If a majority of Eji [j in Ai] disagree with yi, flip Mi.
                let disagreements = Ai.non_trivial_positions()
                    .filter(|j| {
                        let bit = Eji.is_one_at(*j, i).unwrap();
                        debug!("Eji({}, {})={}", *j, i, bit);
                        bit != yi
                    })
                    .count();
                debug!("yi={}, #disagreements={}", yi, disagreements);
                if disagreements > (Ai.weight() / 2) { // TODO OPTIMISE Ai.weight should be constant, ∀i∈[0..N)
                    let update = SparseBinVec::new(code.len(), vec![i]);
                    Mi = &Mi + &update;
                    debug!("Flipped Mi to {}", Mi.get(i).unwrap())
                }
            }

            // Stopping criteria: are the parity-check equations satisfied?
            debug!("Step 3: Stopping criteria");
            let mut all_parity_check_equations_satisfied = true;
            for j in 0..m {
                // sj = Sigma_i∈Bj(Mi mod 2)
                let Bj = code.parity_check_matrix().row(j).unwrap();
                debug!("B{}={}", j, Bj);
                let sigma = Bj
                    .non_trivial_positions()
                    .into_iter()
                    .fold(BinNum::zero(), | acc, el| {
                        let bit = Mi.get(el).unwrap();
                        debug!("Message bit {}={}", el, bit);
                        acc + bit
                    } );
                debug!("sigma={}", sigma);
                if sigma.is_one() {
                    all_parity_check_equations_satisfied = false;
                    debug!("Parity check {} not satisfied", j);
                    break;
                }
            }
            if all_parity_check_equations_satisfied || (iteration == self.Imax) {
                debug!("All parity check equations satisfied or reached {} iteration", self.Imax);
                break;
            } else {
                iteration += 1;
                debug!("Next iteration {}", iteration);
            }
        }
        debug!("Returning {}", Mi);
        Mi
    }
}


#[cfg(test)]
#[path = "./ldpc_spec.rs"]
mod ldpc_spec;