use crate::viewer3d::ScalarField;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldSlice {
    pub axis: SliceAxis,
    pub index: usize,
    pub width: usize,
    pub height: usize,
    pub values: Vec<f64>,
    pub min: f64,
    pub max: f64,
}

/// Extract a 2D slice from a 3D scalar field.
pub fn extract_slice(field: &ScalarField, axis: SliceAxis, index: usize) -> Result<FieldSlice, String> {
    let [nx, ny, nz] = field.shape;
    let (width, height, values) = match axis {
        SliceAxis::Z => {
            if index >= nz {
                return Err(format!("slice index {index} >= nz {nz}"));
            }
            let mut v = Vec::with_capacity(nx * ny);
            for j in 0..ny {
                for i in 0..nx {
                    v.push(field.values[i + j * nx + index * nx * ny]);
                }
            }
            (nx, ny, v)
        }
        SliceAxis::Y => {
            if index >= ny {
                return Err(format!("slice index {index} >= ny {ny}"));
            }
            let mut v = Vec::with_capacity(nx * nz);
            for k in 0..nz {
                for i in 0..nx {
                    v.push(field.values[i + index * nx + k * nx * ny]);
                }
            }
            (nx, nz, v)
        }
        SliceAxis::X => {
            if index >= nx {
                return Err(format!("slice index {index} >= nx {nx}"));
            }
            let mut v = Vec::with_capacity(ny * nz);
            for k in 0..nz {
                for j in 0..ny {
                    v.push(field.values[index + j * nx + k * nx * ny]);
                }
            }
            (ny, nz, v)
        }
    };

    let (min, max) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &x| {
            (lo.min(x), hi.max(x))
        });

    Ok(FieldSlice {
        axis,
        index,
        width,
        height,
        values,
        min,
        max,
    })
}

/// Map normalized value [0,1] to RGBA (blue → cyan → green → yellow → red).
pub fn jet_colormap(t: f64) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    let r = (1.5 - (4.0 * t - 3.0).abs()).clamp(0.0, 1.0);
    let g = (1.5 - (4.0 * t - 2.0).abs()).clamp(0.0, 1.0);
    let b = (1.5 - (4.0 * t - 1.0).abs()).clamp(0.0, 1.0);
    [
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
        255,
    ]
}

pub fn slice_to_rgba(slice: &FieldSlice) -> Vec<u8> {
    let span = (slice.max - slice.min).max(1e-12);
    let mut rgba = Vec::with_capacity(slice.width * slice.height * 4);
    for &v in &slice.values {
        let t = (v - slice.min) / span;
        rgba.extend_from_slice(&jet_colormap(t));
    }
    rgba
}

/// Demo 3D Gaussian blob centred in the unit cube.
pub fn demo_gaussian_field(n: usize) -> ScalarField {
    let mut values = Vec::with_capacity(n * n * n);
    let cx = 0.5;
    let cy = 0.5;
    let cz = 0.5;
    let sigma = 0.15;
    for k in 0..n {
        let z = k as f64 / (n.saturating_sub(1).max(1) as f64);
        for j in 0..n {
            let y = j as f64 / (n.saturating_sub(1).max(1) as f64);
            for i in 0..n {
                let x = i as f64 / (n.saturating_sub(1).max(1) as f64);
                let dx = x - cx;
                let dy = y - cy;
                let dz = z - cz;
                let r2 = dx * dx + dy * dy + dz * dz;
                values.push((-r2 / (2.0 * sigma * sigma)).exp());
            }
        }
    }
    ScalarField {
        shape: [n, n, n],
        values,
        bounds: [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
    }
}
