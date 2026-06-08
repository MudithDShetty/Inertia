//! FFT and inverse FFT (1-D).

use num_complex::Complex64;
use rustfft::FftPlanner;

pub fn fft_1d(x: &[f64]) -> Vec<Complex64> {
    let n = x.len();
    let mut buffer: Vec<Complex64> = x.iter().map(|&v| Complex64::new(v, 0.0)).collect();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    fft.process(&mut buffer);
    buffer
}

pub fn ifft_1d(x: &[Complex64]) -> Vec<Complex64> {
    let n = x.len();
    let mut buffer = x.to_vec();
    let mut planner = FftPlanner::new();
    let ifft = planner.plan_fft_inverse(n);
    ifft.process(&mut buffer);
    let scale = 1.0 / n as f64;
    buffer.iter_mut().for_each(|c| *c *= scale);
    buffer
}

pub fn fft_magnitude(x: &[f64]) -> Vec<f64> {
    fft_1d(x).into_iter().map(|c| c.norm()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_complex::Complex64;

    #[test]
    fn roundtrip_dc() {
        let x = vec![1.0, 1.0, 1.0, 1.0];
        let spec = fft_1d(&x);
        let back = ifft_1d(&spec);
        for (a, b) in x.iter().zip(back.iter()) {
            assert!((a - b.re).abs() < 1e-10);
            assert!(b.im.abs() < 1e-10);
        }
    }

    #[test]
    fn impulse_spectrum_flat() {
        let mut x = vec![0.0; 8];
        x[0] = 1.0;
        let spec = fft_1d(&x);
        for c in spec {
            assert!((c.re - 1.0).abs() < 1e-10);
            assert!(c.im.abs() < 1e-10);
        }
    }
}
