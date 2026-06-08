//! Linear algebra: matmul, solve, inverse, SVD, eigendecomposition.

use crate::tensor::{Array2View, Tensor, TensorError};
use faer::prelude::*;
use faer::{Mat, MatRef, Side};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinalgResult {
    pub data: Vec<f64>,
    pub rows: usize,
    pub cols: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigResult {
    pub eigenvalues_re: Vec<f64>,
    pub eigenvalues_im: Vec<f64>,
    pub eigenvectors: LinalgResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvdResult {
    pub u: LinalgResult,
    pub s: Vec<f64>,
    pub vt: LinalgResult,
}

pub fn matmul(a: &Tensor, b: &Tensor) -> Result<Tensor, TensorError> {
    let a2 = a.as_array2()?;
    let b2 = b.as_array2()?;
    if a2.cols != b2.rows {
        return Err(TensorError::ShapeMismatch {
            expected: a2.cols,
            got: b2.rows,
        });
    }
    let a_mat = to_faer(&a2);
    let b_mat = to_faer(&b2);
    let c = &a_mat * &b_mat;
    Ok(from_faer(&c))
}

pub fn solve(a: &Tensor, b: &Tensor) -> Result<Tensor, MathError> {
    let a2 = a.as_array2()?;
    let b2 = b.as_array2()?;
    if a2.rows != a2.cols {
        return Err(MathError::NotSquare {
            rows: a2.rows,
            cols: a2.cols,
        });
    }
    if b2.rows != a2.rows {
        return Err(MathError::RhsShape {
            a_rows: a2.rows,
            b_rows: b2.rows,
            b_cols: b2.cols,
        });
    }

    let a_mat = to_faer(&a2);
    let b_mat = to_faer(&b2);
    let x = a_mat.partial_piv_lu().solve(b_mat);
    Ok(from_faer(&x))
}

pub fn inverse(a: &Tensor) -> Result<Tensor, MathError> {
    let a2 = a.as_array2()?;
    if a2.rows != a2.cols {
        return Err(MathError::NotSquare {
            rows: a2.rows,
            cols: a2.cols,
        });
    }
    let n = a2.rows;
    let a_mat = to_faer(&a2);
    let ident: Mat<f64> = Mat::identity(n, n);
    let inv = a_mat.partial_piv_lu().solve(ident);
    Ok(from_faer(&inv))
}

pub fn eig(a: &Tensor) -> Result<EigResult, MathError> {
    let a2 = a.as_array2()?;
    if a2.rows != a2.cols {
        return Err(MathError::NotSquare {
            rows: a2.rows,
            cols: a2.cols,
        });
    }
    let a_mat = to_faer(&a2);
    let eigenvalues_re = a_mat.selfadjoint_eigenvalues(Side::Lower);
    let eig = a_mat.selfadjoint_eigendecomposition(Side::Lower);
    let n = a2.rows;

    Ok(EigResult {
        eigenvalues_re,
        eigenvalues_im: vec![0.0; n],
        eigenvectors: from_faer_ref(eig.u()),
    })
}

pub fn svd(a: &Tensor) -> Result<SvdResult, MathError> {
    let a2 = a.as_array2()?;
    let a_mat = to_faer(&a2);
    let svd = a_mat.svd();
    let s = svd.s_diagonal();
    let vt = svd.v().adjoint();

    Ok(SvdResult {
        u: from_faer_ref(svd.u()),
        s: (0..s.nrows()).map(|i| s.read(i)).collect(),
        vt: from_faer_ref(vt),
    })
}

pub fn cholesky(a: &Tensor) -> Result<Tensor, MathError> {
    let a2 = a.as_array2()?;
    if a2.rows != a2.cols {
        return Err(MathError::NotSquare {
            rows: a2.rows,
            cols: a2.cols,
        });
    }
    let a_mat = to_faer(&a2);
    let chol = a_mat
        .cholesky(Side::Lower)
        .map_err(|e| MathError::Faer(e.to_string()))?;
    Ok(from_faer(&chol.compute_l()))
}

/// Benchmark helper: square matmul timing in milliseconds (faer GEMM).
pub fn bench_matmul(n: usize, iters: usize) -> f64 {
    use std::time::Instant;
    let a = Mat::from_fn(n, n, |i, j| ((i + j) % 17) as f64 * 0.01);
    let b = Mat::from_fn(n, n, |i, j| ((i * 3 + j) % 13) as f64 * 0.02);
    let start = Instant::now();
    for _ in 0..iters {
        let _ = &a * &b;
    }
    start.elapsed().as_secs_f64() * 1000.0 / iters as f64
}

fn to_faer(view: &Array2View<'_>) -> Mat<f64> {
    let mut m = Mat::zeros(view.rows, view.cols);
    for i in 0..view.rows {
        for j in 0..view.cols {
            m.write(i, j, view.get(i, j));
        }
    }
    m
}

fn from_faer(m: &Mat<f64>) -> Tensor {
    let lr = from_faer_linalg(m);
    Tensor::from_vec(vec![lr.rows, lr.cols], lr.data).expect("faer matrix converts to tensor")
}

fn from_faer_ref(m: MatRef<'_, f64>) -> LinalgResult {
    let rows = m.nrows();
    let cols = m.ncols();
    let mut data = Vec::with_capacity(rows * cols);
    for i in 0..rows {
        for j in 0..cols {
            data.push(m.read(i, j));
        }
    }
    LinalgResult { data, rows, cols }
}

fn from_faer_linalg(m: &Mat<f64>) -> LinalgResult {
    from_faer_ref(m.as_ref())
}

#[derive(Debug, thiserror::Error)]
pub enum MathError {
    #[error(transparent)]
    Tensor(#[from] TensorError),
    #[error("matrix must be square, got {rows}x{cols}")]
    NotSquare { rows: usize, cols: usize },
    #[error("RHS shape incompatible: A is {a_rows}x?, B is {b_rows}x{b_cols}")]
    RhsShape {
        a_rows: usize,
        b_rows: usize,
        b_cols: usize,
    },
    #[error("linear algebra failure: {0}")]
    Faer(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::Tensor;

    fn m2(data: [[f64; 2]; 2]) -> Tensor {
        Tensor::from_vec(
            vec![2, 2],
            vec![data[0][0], data[0][1], data[1][0], data[1][1]],
        )
        .unwrap()
    }

    #[test]
    fn matmul_2x2() {
        let a = m2([[1.0, 2.0], [3.0, 4.0]]);
        let b = m2([[2.0, 0.0], [1.0, 2.0]]);
        let c = matmul(&a, &b).unwrap();
        assert!((c.as_array2().unwrap().get(0, 0) - 4.0).abs() < 1e-10);
        assert!((c.as_array2().unwrap().get(1, 1) - 8.0).abs() < 1e-10);
    }

    #[test]
    fn solve_and_inverse() {
        let a = m2([[4.0, 1.0], [1.0, 3.0]]);
        let b = Tensor::from_vec(vec![2, 1], vec![1.0, 2.0]).unwrap();
        let x = solve(&a, &b).unwrap();
        let ax = matmul(&a, &x).unwrap();
        assert!((ax.data()[0] - 1.0).abs() < 1e-8);
        assert!((ax.data()[1] - 2.0).abs() < 1e-8);
        let inv = inverse(&a).unwrap();
        let prod = matmul(&a, &inv).unwrap();
        assert!((prod.as_array2().unwrap().get(0, 0) - 1.0).abs() < 1e-8);
    }

    #[test]
    fn symmetric_eig() {
        let a = m2([[2.0, 1.0], [1.0, 2.0]]);
        let e = eig(&a).unwrap();
        assert!((e.eigenvalues_re[0] - 1.0).abs() < 1e-8 || (e.eigenvalues_re[1] - 1.0).abs() < 1e-8);
        assert!((e.eigenvalues_re[0] - 3.0).abs() < 1e-8 || (e.eigenvalues_re[1] - 3.0).abs() < 1e-8);
    }

    #[test]
    fn cholesky_spd() {
        let a = m2([[4.0, 1.0], [1.0, 3.0]]);
        let l = cholesky(&a).unwrap();
        let ll = matmul(&l, &l).unwrap();
        assert!((ll.as_array2().unwrap().get(0, 0) - 4.0).abs() < 1e-8);
    }
}
