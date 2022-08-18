use ldpc::codes::LinearCode;
use sparse_bin_mat::SparseBinMat;

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
// This found a seed of 512, and its output is in parity_check_matrix.alist
// TODO convert this alist format into Rust code that'll construct a SparseBinMat, that'll be
// compiled statically.

// TODO generate many matrixes and evaluate their error correction performance
pub static PARITY_CHECK_MATRIX: SparseBinMat = SparseBinMat::new(
    7,
    vec![vec![0, 1, 2, 4], vec![0, 1, 3, 5], vec![0, 2, 3, 6]]
);
// TODO error[E0010]: allocations are not allowed in statics

pub static CODE_FROM_PARITY: LinearCode = LinearCode::from_parity_check_matrix(PARITY_CHECK_MATRIX);


#[cfg(test)]
#[path = "./ldpc_spec.rs"]
mod ldpc_spec;