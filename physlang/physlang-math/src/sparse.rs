//! CSR sparse matrix (basic matvec).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseMatrixCsr {
    pub rows: usize,
    pub cols: usize,
    pub row_ptr: Vec<usize>,
    pub col_idx: Vec<usize>,
    pub values: Vec<f64>,
}

impl SparseMatrixCsr {
    pub fn from_dense(rows: usize, cols: usize, data: &[f64], zero_tol: f64) -> Self {
        let mut row_ptr = vec![0usize];
        let mut col_idx = Vec::new();
        let mut values = Vec::new();
        for i in 0..rows {
            for j in 0..cols {
                let v = data[i * cols + j];
                if v.abs() > zero_tol {
                    col_idx.push(j);
                    values.push(v);
                }
            }
            row_ptr.push(col_idx.len());
        }
        Self {
            rows,
            cols,
            row_ptr,
            col_idx,
            values,
        }
    }

    pub fn matvec(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.cols);
        let mut y = vec![0.0; self.rows];
        for i in 0..self.rows {
            let start = self.row_ptr[i];
            let end = self.row_ptr[i + 1];
            let mut sum = 0.0;
            for k in start..end {
                sum += self.values[k] * x[self.col_idx[k]];
            }
            y[i] = sum;
        }
        y
    }

    pub fn nnz(&self) -> usize {
        self.values.len()
    }
}

/// Taichi-style sparse grid stub — stores active block indices only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparseGrid3D {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub block_size: usize,
    pub active_blocks: Vec<(usize, usize, usize)>,
}

impl SparseGrid3D {
    pub fn new(nx: usize, ny: usize, nz: usize, block_size: usize) -> Self {
        Self {
            nx,
            ny,
            nz,
            block_size,
            active_blocks: Vec::new(),
        }
    }

    pub fn activate_block(&mut self, bx: usize, by: usize, bz: usize) {
        if !self.active_blocks.contains(&(bx, by, bz)) {
            self.active_blocks.push((bx, by, bz));
        }
    }

    pub fn num_active(&self) -> usize {
        self.active_blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csr_matvec() {
        let dense = vec![1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0];
        let m = SparseMatrixCsr::from_dense(3, 3, &dense, 1e-12);
        let y = m.matvec(&[1.0, 1.0, 1.0]);
        assert!((y[0] - 3.0).abs() < 1e-10);
        assert!((y[1] - 3.0).abs() < 1e-10);
        assert!((y[2] - 4.0).abs() < 1e-10);
    }
}
