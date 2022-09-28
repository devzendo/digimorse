use std::{fs, io};
use std::path::Path;
use log::debug;
use sparse_bin_mat::{SparseBinMat, SparseBinVec};

pub const PARITY_CHECK_MATRIX_ALIST: &'static str = "src/libs/channel_codec/parity_check_matrix.alist";
pub const PARITY_CHECK_MATRIX_RS: &'static str = "src/libs/channel_codec/parity_check_matrix.rs";

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

// Based on from_alist in Daniel Estévez' LDPC-Toolbox, but corrected w.r.t. row/column ordering
// as per Radford M. Neal's LDPC-Codes' pchk-to-alist.c
// Also returns a SparseBinMat.
//
// The alist "spec" at http://www.inference.org.uk/mackay/codes/alist.html is vague, as it talks of
// M and N - very cryptic. (M should be rows? N should be cols?) It's also different to
// Radford Neal's code - possibly Daniel's reader conforms to this.
//
// The alist format, as I interpret it from pchk-to-alist.c:
// num-rows num-cols
// max-row-weight max-col-weight
// [row-weights]    // there are num-rows entries: how many 1's on each row
// [column weights] // there are num-cols entries: how many 1's on each col
// for each row 0..num-rows:
//   [1-based indices of 1s in the row]
// for each column 0..num-cols:
//   [1-based indices of 1s in the column]
pub fn from_alist(alist: &str) -> Result<SparseBinMat, String> {
    let mut alist = alist.split('\n');
    let sizes = alist
        .next()
        .ok_or_else(|| String::from("alist first line not found"))?;
    let mut sizes = sizes.split_whitespace();
    let nrows = sizes
        .next()
        .ok_or_else(|| String::from("alist first line (dimensions) does not contain enough elements"))?
        .parse()
        .map_err(|_| String::from("nrows is not a number"))?;
    let ncols = sizes
        .next()
        .ok_or_else(|| String::from("alist first line (dimensions) does not contain enough elements"))?
        .parse()
        .map_err(|_| String::from("ncols is not a number"))?;
    let mut h = SparseBinMat::zeros(nrows, ncols);
    alist.next(); // skip max weights
    alist.next(); // skip row weights
    alist.next(); // skip column weights
    // position of 1's in each row
    for row in 0..nrows {
        let row_data = alist
            .next()
            .ok_or_else(|| String::from("alist does not contain expected number of lines (expecting row data)"))?;
        let row_data = row_data.split_whitespace();
        for col in row_data {
            let col: usize = col
                .parse()
                .map_err(|_| String::from("col value is not a number"))?;
            h = h.emplace_at(1, row, col -1);
        }
    }
    // position of 1's in eech column
    // may not be necessary
    for col in 0..ncols {
        let col_data = alist
            .next()
            .ok_or_else(|| String::from("alist does not contain expected number of lines (expecting column data)"))?;
        let col_data = col_data.split_whitespace();
        for row in col_data {
            let row: usize = row
                .parse()
                .map_err(|_| String::from("row value is not a number"))?;
            h = h.emplace_at(1, row - 1, col);
        }
    }
    Ok(h)
}

/*
Example small_parity_check_matrix.pchk:
Parity check matrix in small_parity_check_matrix.pchk (dense format):

 1 0 1 1 1 1
 1 1 1 1 1 0
 1 1 1 0 0 1
 0 1 0 1 1 1

And small_parity_check_matrix.alist:
4 6
5 3
5 5 4 4
3 3 3 3 3 3
1 3 4 5 6
1 2 3 4 5
1 2 3 6 0
2 4 5 6 0
1 2 3
2 3 4
1 2 3
1 2 4
1 2 4
1 3 4

 */
pub fn load_parity_check_matrix() -> Result<SparseBinMat, String> {
    let sbm = from_alist(fs::read_to_string(PARITY_CHECK_MATRIX_ALIST).unwrap().as_str())?;
    debug!("SparseBinMat is ({}, {})", sbm.number_of_rows(), sbm.number_of_columns());
    Ok(sbm)
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


#[cfg(test)]
#[path = "./ldpc_util_spec.rs"]
mod ldpc_util_spec;