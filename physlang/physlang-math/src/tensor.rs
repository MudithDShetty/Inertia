//! Dense N-dimensional tensor backed by a flat row-major buffer.

use ndarray::{Array, ArrayD, IxDyn};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Tensor {
    pub shape: Vec<usize>,
    data: Vec<f64>,
}

impl fmt::Debug for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tensor")
            .field("shape", &self.shape)
            .field("len", &self.data.len())
            .finish()
    }
}

impl Tensor {
    pub fn from_vec(shape: Vec<usize>, data: Vec<f64>) -> Result<Self, TensorError> {
        let n: usize = shape.iter().product();
        if data.len() != n {
            return Err(TensorError::ShapeMismatch {
                expected: n,
                got: data.len(),
            });
        }
        Ok(Self { shape, data })
    }

    pub fn zeros(shape: Vec<usize>) -> Self {
        let n: usize = shape.iter().product();
        Self {
            shape,
            data: vec![0.0; n],
        }
    }

    pub fn ones(shape: Vec<usize>) -> Self {
        let n: usize = shape.iter().product();
        Self {
            shape,
            data: vec![1.0; n],
        }
    }

    pub fn from_array2(m: &Array2View<'_>) -> Self {
        Self {
            shape: vec![m.rows, m.cols],
            data: m.data.to_vec(),
        }
    }

    pub fn as_array(&self) -> ArrayD<f64> {
        Array::from_shape_vec(IxDyn(&self.shape), self.data.clone())
            .expect("tensor shape matches data length")
    }

    pub fn as_array2(&self) -> Result<Array2View<'_>, TensorError> {
        if self.shape.len() != 2 {
            return Err(TensorError::NotMatrix {
                ndim: self.shape.len(),
            });
        }
        Ok(Array2View {
            rows: self.shape[0],
            cols: self.shape[1],
            data: &self.data,
        })
    }

    pub fn data(&self) -> &[f64] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [f64] {
        &mut self.data
    }

    pub fn numel(&self) -> usize {
        self.data.len()
    }
}

pub struct Array2View<'a> {
    pub rows: usize,
    pub cols: usize,
    pub data: &'a [f64],
}

impl Array2View<'_> {
    pub fn to_ndarray(&self) -> ndarray::Array2<f64> {
        ndarray::Array2::from_shape_vec((self.rows, self.cols), self.data.to_vec())
            .expect("matrix shape matches data")
    }

    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row * self.cols + col]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TensorError {
    #[error("shape product {expected} != data length {got}")]
    ShapeMismatch { expected: usize, got: usize },
    #[error("expected 2-D matrix, got {ndim}-D tensor")]
    NotMatrix { ndim: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeros_and_ones() {
        let z = Tensor::zeros(vec![2, 3]);
        assert_eq!(z.numel(), 6);
        assert!(z.data().iter().all(|&x| x == 0.0));
        let o = Tensor::ones(vec![2, 2]);
        assert!(o.data().iter().all(|&x| x == 1.0));
    }
}
