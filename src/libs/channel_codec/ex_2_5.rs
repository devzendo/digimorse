use sparse_bin_mat::SparseBinMat;

// From "Iterative Error Correction", Example 2.5 "A regular parity-check matrix, with
// Wc = 2 and Wr = 3"
pub fn example_2_5_parity_check_matrix() -> SparseBinMat {
    SparseBinMat::new(6, vec![
        vec![0, 1, 3],
        vec![1, 2, 4],
        vec![0, 4, 5],
        vec![2, 3, 5],
    ])
}

#[cfg(test)]
#[path = "./ex_2_5_spec.rs"]
mod ex_2_5_spec;
