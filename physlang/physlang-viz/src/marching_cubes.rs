//! Marching-cubes isosurface stub — coarse mesh from `ScalarField`.

use crate::viewer3d::ScalarField;

#[derive(Debug, Clone, PartialEq)]
pub struct IsoMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// Extract a coarse isosurface at `isovalue` (downsampled grid).
pub fn extract_isosurface_stub(field: &ScalarField, isovalue: f64, step: usize) -> IsoMesh {
    let step = step.max(1);
    let [nx, ny, nz] = field.shape;
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let sample = |ix: usize, iy: usize, iz: usize| -> f64 {
        field.values[ix + iy * nx + iz * nx * ny]
    };

    let grad = |ix: usize, iy: usize, iz: usize| -> [f32; 3] {
        let ix0 = ix.saturating_sub(1);
        let ix1 = (ix + 1).min(nx - 1);
        let iy0 = iy.saturating_sub(1);
        let iy1 = (iy + 1).min(ny - 1);
        let iz0 = iz.saturating_sub(1);
        let iz1 = (iz + 1).min(nz - 1);
        let gx = (sample(ix1, iy, iz) - sample(ix0, iy, iz)) as f32;
        let gy = (sample(ix, iy1, iz) - sample(ix, iy0, iz)) as f32;
        let gz = (sample(ix, iy, iz1) - sample(ix, iy, iz0)) as f32;
        let len = (gx * gx + gy * gy + gz * gz).sqrt().max(1e-6);
        [gx / len, gy / len, gz / len]
    };

    let to_world = |ix: usize, iy: usize, iz: usize| -> [f32; 3] {
        let [[x0, y0, z0], [x1, y1, z1]] = field.bounds;
        let fx = if nx > 1 {
            ix as f64 / (nx - 1) as f64
        } else {
            0.0
        };
        let fy = if ny > 1 {
            iy as f64 / (ny - 1) as f64
        } else {
            0.0
        };
        let fz = if nz > 1 {
            iz as f64 / (nz - 1) as f64
        } else {
            0.0
        };
        [
            (x0 + fx * (x1 - x0)) as f32,
            (y0 + fy * (y1 - y0)) as f32,
            (z0 + fz * (z1 - z0)) as f32,
        ]
    };

    for iz in (0..nz.saturating_sub(step)).step_by(step) {
        for iy in (0..ny.saturating_sub(step)).step_by(step) {
            for ix in (0..nx.saturating_sub(step)).step_by(step) {
                let v = sample(ix, iy, iz);
                if v >= isovalue {
                    continue;
                }
                // Stub: emit a small quad on the cell face nearest to isovalue crossing.
                let vxp = sample((ix + step).min(nx - 1), iy, iz);
                if v < isovalue && vxp >= isovalue {
                    let base = vertices.len() as u32;
                    let p0 = to_world(ix, iy, iz);
                    let p1 = to_world((ix + step).min(nx - 1), iy, iz);
                    let p2 = to_world((ix + step).min(nx - 1), (iy + step).min(ny - 1), iz);
                    let p3 = to_world(ix, (iy + step).min(ny - 1), iz);
                    let n = grad(ix, iy, iz);
                    vertices.extend([p0, p1, p2, p3]);
                    normals.extend([n, n, n, n]);
                    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
    }

    IsoMesh {
        vertices,
        normals,
        indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_slice::demo_gaussian_field;

    #[test]
    fn isosurface_stub_non_empty() {
        let field = demo_gaussian_field(16);
        let mesh = extract_isosurface_stub(&field, 0.3, 2);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
    }
}
