use sparse_bin_mat::{SparseBinMat, SparseBinSlice};

// TODO submit as a PR to sparse-binary-matrix?
pub trait ColumnAccess {
    fn column(&self, column: usize) -> Option<SparseBinSlice>;
}

impl ColumnAccess for SparseBinMat {
    fn column(&self, column: usize) -> Option<SparseBinSlice> {
        if column < self.number_of_columns() {
            let mut column_positions: Vec<usize> = vec![];
            (0..self.number_of_rows()).for_each(|y| if self.row(y).unwrap().get(column).is_some() { column_positions.push(y) } );
            // Some(SparseBinVec::new(self.number_of_columns(), column_positions).as_view())
            Some(SparseBinSlice::new(0, &[]))
        } else {
            None
        }

    }
}

