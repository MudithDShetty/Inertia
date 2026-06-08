//! Numerical integration and finite-difference operators.

pub fn trapezoid(y: &[f64], dx: f64) -> f64 {
    if y.len() < 2 {
        return 0.0;
    }
    let mut sum = 0.5 * (y[0] + y[y.len() - 1]);
    for v in &y[1..y.len() - 1] {
        sum += *v;
    }
    sum * dx
}

pub fn simpson(y: &[f64], dx: f64) -> f64 {
    let n = y.len();
    assert!(n >= 3 && n % 2 == 1, "Simpson requires odd number of points");
    let mut sum = y[0] + y[n - 1];
    for i in (1..n - 1).step_by(2) {
        sum += 4.0 * y[i];
    }
    for i in (2..n - 1).step_by(2) {
        sum += 2.0 * y[i];
    }
    sum * dx / 3.0
}

pub fn gauss_legendre_2(f: impl Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    let nodes = [-0.5773502691896257, 0.5773502691896257];
    let weights = [1.0, 1.0];
    let mid = 0.5 * (a + b);
    let half = 0.5 * (b - a);
    nodes
        .iter()
        .zip(weights.iter())
        .map(|(&x, &w)| w * f(mid + half * x))
        .sum::<f64>()
        * half
}

pub fn gradient_1d(f: &[f64], h: f64) -> Vec<f64> {
    let n = f.len();
    let mut g = vec![0.0; n];
    if n < 2 {
        return g;
    }
    g[0] = (f[1] - f[0]) / h;
    for i in 1..n - 1 {
        g[i] = (f[i + 1] - f[i - 1]) / (2.0 * h);
    }
    g[n - 1] = (f[n - 1] - f[n - 2]) / h;
    g
}

pub fn laplacian_1d(f: &[f64], h: f64) -> Vec<f64> {
    let n = f.len();
    let mut d2 = vec![0.0; n];
    if n < 3 {
        return d2;
    }
    let h2 = h * h;
    for i in 1..n - 1 {
        d2[i] = (f[i + 1] - 2.0 * f[i] + f[i - 1]) / h2;
    }
    d2
}

pub fn divergence_2d(vx: &[f64], vy: &[f64], nx: usize, ny: usize, hx: f64, hy: f64) -> Vec<f64> {
    assert_eq!(vx.len(), nx * ny);
    assert_eq!(vy.len(), nx * ny);
    let mut div = vec![0.0; nx * ny];
    for j in 0..ny {
        for i in 0..nx {
            let idx = j * nx + i;
            let dvx_dx = if i == 0 {
                (vx[idx + 1] - vx[idx]) / hx
            } else if i == nx - 1 {
                (vx[idx] - vx[idx - 1]) / hx
            } else {
                (vx[idx + 1] - vx[idx - 1]) / (2.0 * hx)
            };
            let dvy_dy = if j == 0 {
                (vy[idx + nx] - vy[idx]) / hy
            } else if j == ny - 1 {
                (vy[idx] - vy[idx - nx]) / hy
            } else {
                (vy[idx + nx] - vy[idx - nx]) / (2.0 * hy)
            };
            div[idx] = dvx_dx + dvy_dy;
        }
    }
    div
}

/// Christoffel symbols stub — returns zeros for flat Euclidean metric.
pub fn christoffel_flat(_dim: usize) -> Vec<f64> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrate_x_squared() {
        let a = 0.0;
        let b = 1.0;
        let n = 101;
        let dx = (b - a) / (n - 1) as f64;
        let y: Vec<f64> = (0..n).map(|i| {
            let x = a + dx * i as f64;
            x * x
        }).collect();
        let trap = trapezoid(&y, dx);
        assert!((trap - 1.0 / 3.0).abs() < 0.001);
        let simp = simpson(&y, dx);
        assert!((simp - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn laplacian_quadratic() {
        let h = 0.1;
        let f: Vec<f64> = (0..11).map(|i| (i as f64 * h).powi(2)).collect();
        let d2 = laplacian_1d(&f, h);
        for i in 1..10 {
            assert!((d2[i] - 2.0).abs() < 0.05);
        }
    }
}
