// use log::debug;
use sparse_bin_mat::{BinNum, SparseBinMat, SparseBinVec};

// TODO submit as a PR to sparse-binary-matrix? Unsure how to get this to return an Option<SparseBinSlice>.
pub trait ColumnAccess {
    fn column(&self, column: usize) -> Option<SparseBinVec>;
}

impl ColumnAccess for SparseBinMat {
    fn column(&self, column: usize) -> Option<SparseBinVec> {
        if column < self.number_of_columns() {
            let mut column_positions: Vec<usize> = vec![];
            (0..self.number_of_rows()).for_each(|y| {
                let bit = self.row(y).unwrap().is_one_at(column).unwrap();
                // debug!("y={}, row[{}] = {}, matrix[{}, {}]={:?} {}", y, y, self.row(y).unwrap(), y, column, self.row(y).unwrap().get(column), bit);
                if bit { column_positions.push(y) }
            });
            Some(SparseBinVec::try_new(self.number_of_rows(), column_positions).unwrap())
        } else {
            None
        }
    }
}

