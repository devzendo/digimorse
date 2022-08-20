extern crate hamcrest2;

#[cfg(test)]
mod ldpc_spec {
    use std::{env, fs, io};
    use std::path::Path;
    use hamcrest2::prelude::*;
    use ldpc_toolbox::sparse;
    use ldpc_toolbox::sparse::SparseMatrix;
    use log::{debug, error, info};
    use pretty_hex::*;
    use sparse_bin_mat::SparseBinMat;
    use crate::libs::util::util::vec_to_array;

    #[ctor::ctor]
    fn before_each() {
        env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {}

    fn sparsematrix_to_sparsebinmat(source: SparseMatrix) -> SparseBinMat {
        let mut destination = SparseBinMat::zeros(source.num_rows(), source.num_cols());
        for row in 0 .. source.num_rows() {
            for col in 0 .. source.num_cols() {
                if source.contains(row, col) {
                    destination = destination.emplace_at(1, row, col);
                }
            }
        }
        destination
    }

    // Given a matrix and an output filename (ending in .dot), create the output file for graphviz'
    // dot to convert to a suitable output file e.g.
    // dot -Tpng my_graph_file.dot -o my_graph_file.png
    fn draw_tanner_graph(source: &SparseBinMat, output_filename: &str) -> io::Result<()> {
        let mut dot = String::new();
        dot += "
graph G {
   ranksep = 1.2;
   nodesep = 1.2;
   splines=false;
   rankdir = LR;
   peripheries = 0;
   subgraph cluster_checks {
      node[shape=square, style=filled]
";
        // check nodes (one per row)
        for row in 0 .. source.number_of_rows() {
            dot += format!("      check{} [label=\"\",fillcolor=lightgray]\n", row + 1).as_str();
        }
        dot += "      ";
        for row in 0 .. source.number_of_rows() {
            dot += format!("check{}", row + 1).as_str();
            if row != source.number_of_rows() - 1 {
                dot += "--";
            }
        }
        dot += "  [style=invis]";
        dot += "
   }
   subgraph cluster_padding1 {
      color=invis;
      a12m [style=invisible]
   }
   subgraph cluster_padding2 {
      color=invis;
      a22m [style=invisible]
   }
   subgraph cluster_bits {
      node[shape=circle, style=filled]
";
        // bit nodes (one per column)
        for col in 0 .. source.number_of_columns() {
            dot += format!("      bit{} [label=\"\",fillcolor=white]\n", col + 1).as_str();
        }
        dot += "      ";
        for col in 0 .. source.number_of_columns() {
            dot += format!("bit{}", col + 1).as_str();
            if col != source.number_of_columns() - 1 {
                dot += "--";
            }
        }
        dot += " [style=invis]
   }\n";
        // edges
        for row in 0 .. source.number_of_rows() {
            for col in 0 .. source.number_of_columns() {
                if source.is_one_at(row, col).unwrap() {
                    dot += format!("   check{}--bit{} [constraint=false]\n", row + 1, col + 1).as_str();
                }
            }
        }
        dot += "}
";
        fs::write(Path::new(output_filename), dot)
    }

    // Given a matrix and an output filename (ending in .rs), create Rust code to instantiate
    // the matrix as a SparseBinMat.
    fn write_rust_generator(source: &SparseBinMat, output_filename: &str) -> io::Result<()> {
        let mut code = String::new();
        fs::write(Path::new(output_filename), code)
    }
    
    fn load_parity_check_matrix() -> sparse::Result<SparseBinMat> {
        let sm = SparseMatrix::from_alist(fs::read_to_string("src/libs/channel_codec/parity_check_matrix.alist").unwrap().as_str())?;
        Ok(sparsematrix_to_sparsebinmat(sm))
    }
    
    #[test]
    #[ignore]
    fn load_alist_into_sparsebinmat() {
        let sm = load_parity_check_matrix();
        info!("{}", sm.unwrap().as_json().unwrap());
    }

    #[test]
    #[ignore]
    fn draw_example_2_5_tanner_graph() {
        // From "Iterative Error Correction", Example 2.5 "A regular parity-check matrix, with
        // Wc = 2 and Wr = 3"
        let ex2_5 = SparseBinMat::new(6, vec![
            vec![0, 1, 3],
            vec![1, 2, 4],
            vec![0, 4, 5],
            vec![2, 3, 5],
        ]);
        draw_tanner_graph(&ex2_5, "/tmp/example_2_5.dot");
    }

    #[test]
    #[ignore]
    fn draw_parity_check_matrix_as_tanner_graph() {
        let sm = load_parity_check_matrix();
        draw_tanner_graph(&sm.unwrap(), "/tmp/digimorse_parity_check_matrix.dot");
        // dot -Tpng /tmp/digimorse_parity_check_matrix.dot -o /tmp/digimorse_parity_check_matrix.png
        // takes a few minutes to generate, complains about being too big, and scaling...
        // and is quite unreadable!
    }

    #[test]
    #[ignore]
    fn generate_rust_for_parity_check_matrix() {
        let sm = load_parity_check_matrix();
        write_rust_generator(&sm.unwrap(), "src/libs/channel_codec/parity_check_matrix.rs");
    }
}
