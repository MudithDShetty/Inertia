//! PhysicsMath — dense linear algebra, FFT, and numerical calculus.

mod calc;
mod fft;
mod kernel;
mod linalg;
mod sparse;
mod tensor;

pub use calc::{
    christoffel_flat, divergence_2d, gauss_legendre_2, gradient_1d, laplacian_1d, simpson,
    trapezoid,
};
pub use fft::{fft_1d, fft_magnitude, ifft_1d};
pub use kernel::{global_kernel_cache, CachedKernel, KernelCache, KernelOp};
pub use linalg::{
    bench_matmul, cholesky, eig, inverse, matmul, solve, EigResult, LinalgResult, MathError,
    SvdResult,
};
pub use sparse::{SparseGrid3D, SparseMatrixCsr};
pub use tensor::{Array2View, Tensor, TensorError};
