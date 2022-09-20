use std::{fs, io};
use std::path::Path;
use ldpc_toolbox::sparse;
use ldpc_toolbox::sparse::SparseMatrix;
use sparse_bin_mat::{SparseBinMat, SparseBinVec};

pub const PARITY_CHECK_MATRIX_ALIST: &'static str = "src/libs/channel_codec/parity_check_matrix.alist";
pub const PARITY_CHECK_MATRIX_RS: &'static str = "src/libs/channel_codec/parity_check_matrix.rs";

// I use Radford M. Neal's LDPC-Codes make-ldpc and pchk-to-alist tools to construct the parity
// check matrix. This is then saved into an alist file, reloaded, and passed in here, to convert
// into the ldpc SparseBinMat, from which I'll construct Rust code to instantiate at runtime.
pub fn sparsematrix_to_sparsebinmat(source: SparseMatrix) -> SparseBinMat {
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
pub fn draw_tanner_graph(source: &SparseBinMat, output_filename: &str) -> io::Result<()> {
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

pub fn load_parity_check_matrix() -> sparse::Result<SparseBinMat> {
    let sm = SparseMatrix::from_alist(fs::read_to_string(PARITY_CHECK_MATRIX_ALIST).unwrap().as_str())?;
    Ok(sparsematrix_to_sparsebinmat(sm))
}

// Given a vector, construct a displayable representation of it
pub fn sparsebinvec_to_display(vec: &SparseBinVec) -> String {
    let mut out = String::new();
    let mut dense = vec.iter_dense();
    loop {
        let maybe_bit = dense.next();
        match maybe_bit {
            None => { break }
            Some(bit) => {
                out.push(if bit.is_one() { '1' } else { '0' });
            }
        }
    }
    out
}

// Given a matrix, construct a displayable representation of it
pub fn display_matrix(source: &SparseBinMat) -> Vec<String> {
    let mut out = vec![];
    for row in 0 .. source.number_of_rows() {
        let mut line = String::new();
        for col in 0 .. source.number_of_columns() {
            if source.is_one_at(row, col).unwrap() {
                line += "1";
            } else {
                line += "0";
            }
            line += " ";
        }
        out.push(line);
    }
    out
}

// Given a matrix, and a variable name, construct Python instantiation of a numpy.array of it
pub fn display_numpy_matrix(source: &SparseBinMat, variable_name: &str) -> Vec<String> {
    let mut out = vec![];
    out.push(format!("{} = np.array([", variable_name));
    for row in 0 .. source.number_of_rows() {
        let mut line = String::new();
        line += "              [";
        for col in 0 .. source.number_of_columns() {
            if source.is_one_at(row, col).unwrap() {
                line += "1";
            } else {
                line += "0";
            }
            if col != source.number_of_columns() - 1 {
                line += ", ";
            }
        }
        line += "]";
        if row != source.number_of_rows() - 1 {
            line += ",";
        }
        out.push(line);
    }
    out.push("])".to_string());
    out
}

// Given a matrix and an output filename (ending in .rs), create Rust code to instantiate
// the matrix as a SparseBinMat.
pub fn generate_rust_for_matrix(source: &SparseBinMat, source_name: &str, output_filename: &str) -> io::Result<()> {
    let mut code = String::new();
    code += "// Autogenerated from ";
    code += source_name;
    code += "\n";
    code += "extern crate lazy_static;\n";
    code += "use lazy_static::lazy_static;\n";
    code += "use ldpc::codes::LinearCode;\n";
    code += "use sparse_bin_mat::SparseBinMat;\n";
    code += "\n";
    code += "lazy_static! {\n";
    code += "  pub static ref LDPC: LinearCode = LinearCode::from_parity_check_matrix(SparseBinMat::new(\n";
    code += "    ";
    code += &source.number_of_columns().to_string().as_str();
    code += ",\n";
    code += "    vec![\n";

    for row in 0 .. source.number_of_rows() {
        code += "      vec![";
        for col in 0 .. source.number_of_columns() {
            if source.is_one_at(row, col).unwrap() {
                code += format!("{}, ", col).as_str();
            }
        }
        code += "],\n";
    }

    code += "    ]\n";
    code += "  ));\n";
    code += "}\n";
    fs::write(Path::new(output_filename), code)
}

