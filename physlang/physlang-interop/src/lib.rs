//! C/Fortran legacy FFI bridge.

use std::ffi::c_int;

extern "C" {
    fn phys_daxpy(n: c_int, alpha: f64, x: *const f64, y: *mut f64);
    fn phys_dot(n: c_int, x: *const f64, y: *const f64) -> f64;
    fn phys_fortran_pi() -> f64;
}

pub fn daxpy(alpha: f64, x: &[f64], y: &mut [f64]) {
    assert_eq!(x.len(), y.len());
    let n = x.len() as c_int;
    unsafe {
        phys_daxpy(n, alpha, x.as_ptr(), y.as_mut_ptr());
    }
}

pub fn dot(x: &[f64], y: &[f64]) -> f64 {
    assert_eq!(x.len(), y.len());
    let n = x.len() as c_int;
    unsafe { phys_dot(n, x.as_ptr(), y.as_ptr()) }
}

pub fn legacy_pi() -> f64 {
    unsafe { phys_fortran_pi() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daxpy_matches_blas_semantics() {
        let x = vec![1.0, 2.0, 3.0];
        let mut y = vec![4.0, 5.0, 6.0];
        daxpy(2.0, &x, &mut y);
        assert!((y[0] - 6.0).abs() < 1e-12);
        assert!((y[1] - 9.0).abs() < 1e-12);
        assert!((y[2] - 12.0).abs() < 1e-12);
    }

    #[test]
    fn dot_product() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![4.0, 5.0, 6.0];
        assert!((dot(&x, &y) - 32.0).abs() < 1e-12);
    }
}
