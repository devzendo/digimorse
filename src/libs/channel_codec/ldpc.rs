// A (240,126) Low-Density Parity-Check code, giving 114 bits of redundant parity information.
// Characteristics required: (From section 3.4 of "Iterative Error Correction", Prof. Sarah J.
// Johnson. All page references are to this book.)
// * At least a girth of 6: there should be no 4-cycles in the Tanner graph.
// * Wc (column weight) of 3 (see p. 77)
// * Wr (row weight) of 6: R=0.525, so R =~ 1 - Wc/Wr; 0.525 =~ 1 - 3/Wr; Wr =~ 6.316 ?? Say 6?
// * Regular (irregular could have an improved threshold performance; however the library I'm using
//   only supports regular).
// * Randomly-allocated: "in many cases, randomly allocating the entries in H will produce a
//   reasonable LDPC code" (p75)
// Parity check matrix:
// Number of columns=number of codeword bits (240)
// Number of rows=number of message bits (126 = 112 encoder bits + 14 CRC bits)

// Using ldpc-toolbox to generate a Mackay-Neal construction, with the following arguments:
// ldpc-toolbox mackay-neal 126 240 6 3 0 --uniform --min-girth 8 --girth-trials 10000 --search
// This found a seed of 512, and its output is in parity_check_matrix.alist.
// The alist file is read and converted into generated Rust code in parity_check_matrix.rs
// The code to do this conversion is the (ignored, manually invoked) test code in ldpc_spec.rs,
// test generate_rust_for_parity_check_matrix().

// TODO generate many matrixes and evaluate their error correction performance

use log::info;
use metered::time_source::{Instant, StdInstant};
use super::parity_check_matrix::LDPC;

// Just to start the lazy_static, and log how long it takes to initialise.
pub fn init_ldpc() {
    let ldpc_init_duration = StdInstant::now();
    info!("LDPC({}, {}) initialised", LDPC.parity_check_matrix().number_of_columns(), LDPC.parity_check_matrix().number_of_rows());
    info!("LDPC initialised in {}ms", ldpc_init_duration.elapsed_time());
}

#[cfg(test)]
#[path = "./ldpc_spec.rs"]
mod ldpc_spec;