//! Marching-cubes isosurface extraction from `ScalarField`.

#[path = "marching_cubes_tables.rs"]
mod tables;

use crate::viewer3d::ScalarField;
use tables::{EDGE_TABLE, TRI_TABLE, EDGE_ENDPOINTS};

#[derive(Debug, Clone, PartialEq)]
pub struct IsoMesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// Extract an isosurface at `isovalue` using marching cubes.
pub fn extract_isosurface(field: &ScalarField, isovalue: f64) -> IsoMesh {
    extract_isosurface_stepped(field, isovalue, 1)
}

/// Downsampled marching cubes (step >= 1 skips voxels).
pub fn extract_isosurface_stepped(field: &ScalarField, isovalue: f64, step: usize) -> IsoMesh {
    let step = step.max(1);
    let [nx, ny, nz] = field.shape;
    if nx < 2 || ny < 2 || nz < 2 {
        return IsoMesh {
            vertices: vec![],
            normals: vec![],
            indices: vec![],
        };
    }

    let sample = |ix: usize, iy: usize, iz: usize| -> f64 {
        field.values[ix + iy * nx + iz * nx * ny]
    };

    let grad_at = |ix: usize, iy: usize, iz: usize| -> [f32; 3] {
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
        [-gx / len, -gy / len, -gz / len]
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

    let interp = |p1: [f32; 3], v1: f64, p2: [f32; 3], v2: f64| -> [f32; 3] {
        if (v1 - v2).abs() < 1e-12 {
            return p1;
        }
        let t = ((isovalue - v1) / (v2 - v1)) as f32;
        [
            p1[0] + t * (p2[0] - p1[0]),
            p1[1] + t * (p2[1] - p1[1]),
            p1[2] + t * (p2[2] - p1[2]),
        ]
    };

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    for iz in (0..nz - 1).step_by(step) {
        for iy in (0..ny - 1).step_by(step) {
            for ix in (0..nx - 1).step_by(step) {
                let ix1 = (ix + step).min(nx - 1);
                let iy1 = (iy + step).min(ny - 1);
                let iz1 = (iz + step).min(nz - 1);

                let v = [
                    sample(ix, iy, iz),
                    sample(ix1, iy, iz),
                    sample(ix1, iy1, iz),
                    sample(ix, iy1, iz),
                    sample(ix, iy, iz1),
                    sample(ix1, iy, iz1),
                    sample(ix1, iy1, iz1),
                    sample(ix, iy1, iz1),
                ];
                let p = [
                    to_world(ix, iy, iz),
                    to_world(ix1, iy, iz),
                    to_world(ix1, iy1, iz),
                    to_world(ix, iy1, iz),
                    to_world(ix, iy, iz1),
                    to_world(ix1, iy, iz1),
                    to_world(ix1, iy1, iz1),
                    to_world(ix, iy1, iz1),
                ];

                let mut cube_index = 0u8;
                for (i, &val) in v.iter().enumerate() {
                    if val < isovalue {
                        cube_index |= 1 << i;
                    }
                }
                if cube_index == 0 || cube_index == 255 {
                    continue;
                }

                let edge_mask = EDGE_TABLE[cube_index as usize];
                if edge_mask == 0 {
                    continue;
                }

                let mut vertlist = [[0.0f32; 3]; 12];
                for e in 0..12 {
                    if (edge_mask & (1 << e)) != 0 {
                        let (a, b) = EDGE_ENDPOINTS[e];
                        vertlist[e] = interp(p[a], v[a], p[b], v[b]);
                    }
                }

                let gix = (ix + ix1) / 2;
                let giy = (iy + iy1) / 2;
                let giz = (iz + iz1) / 2;
                let n = grad_at(gix, giy, giz);

                let tri_row = TRI_TABLE[cube_index as usize];
                let mut t = 0usize;
                while t < 16 && tri_row[t] >= 0 {
                    let e0 = tri_row[t] as usize;
                    let e1 = tri_row[t + 1] as usize;
                    let e2 = tri_row[t + 2] as usize;
                    let base = vertices.len() as u32;
                    vertices.push(vertlist[e0]);
                    vertices.push(vertlist[e1]);
                    vertices.push(vertlist[e2]);
                    normals.extend([n, n, n]);
                    indices.extend([base, base + 1, base + 2]);
                    t += 3;
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

/// Legacy alias — now delegates to real marching cubes.
pub fn extract_isosurface_stub(field: &ScalarField, isovalue: f64, step: usize) -> IsoMesh {
    extract_isosurface_stepped(field, isovalue, step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_slice::demo_gaussian_field;

    #[test]
    fn isosurface_non_empty() {
        let field = demo_gaussian_field(16);
        let mesh = extract_isosurface(&field, 0.3);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert!(!mesh.indices.is_empty());
    }

    #[test]
    fn isosurface_stub_alias() {
        let field = demo_gaussian_field(16);
        let mesh = extract_isosurface_stub(&field, 0.3, 1);
        assert!(!mesh.vertices.is_empty());
    }
}
