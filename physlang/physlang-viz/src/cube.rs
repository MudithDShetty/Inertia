//! Gaussian CUBE volumetric grid parser (density, MO, ESP).

use crate::elements::symbol_to_z;
use crate::viewer3d::ScalarField;

#[derive(Debug, Clone, PartialEq)]
pub struct CubeVolume {
    pub title: [String; 2],
    pub origin: [f64; 3],
    pub shape: [usize; 3],
    pub voxel_vectors: [[f64; 3]; 3],
    pub atoms: Vec<CubeAtom>,
    pub field: ScalarField,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CubeAtom {
    pub atomic_number: u8,
    pub charge: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Parse a Gaussian `.cube` file into a scalar field for isosurface rendering.
pub fn parse_cube(source: &str) -> Result<CubeVolume, String> {
    let mut lines = source.lines().filter(|l| !l.trim().is_empty());
    let title0 = lines.next().unwrap_or("cube").trim().to_string();
    let title1 = lines.next().unwrap_or("").trim().to_string();

    let header = lines
        .next()
        .ok_or("cube: missing atom/origin line")?;
    let hdr: Vec<&str> = header.split_whitespace().collect();
    if hdr.len() < 4 {
        return Err("cube: bad atom/origin line".into());
    }
    let natom: i32 = hdr[0].parse().map_err(|_| "cube: bad atom count")?;
    let origin: [f64; 3] = [
        hdr[1].parse().map_err(|_| "cube: bad ox")?,
        hdr[2].parse().map_err(|_| "cube: bad oy")?,
        hdr[3].parse().map_err(|_| "cube: bad oz")?,
    ];

    let mut shape = [0usize; 3];
    let mut voxel_vectors = [[0.0f64; 3]; 3];
    for axis in 0..3 {
        let row = lines
            .next()
            .ok_or_else(|| format!("cube: missing grid axis {axis}"))?;
        let p: Vec<&str> = row.split_whitespace().collect();
        if p.len() < 4 {
            return Err(format!("cube: bad grid line {axis}"));
        }
        let n: i32 = p[0].parse().map_err(|_| "cube: bad nx/ny/nz")?;
        if n <= 0 {
            return Err("cube: grid dimension must be positive".into());
        }
        shape[axis] = n as usize;
        voxel_vectors[axis] = [
            p[1].parse().map_err(|_| "cube: bad voxel")?,
            p[2].parse().map_err(|_| "cube: bad voxel")?,
            p[3].parse().map_err(|_| "cube: bad voxel")?,
        ];
    }

    let n_atoms = if natom <= 0 {
        0
    } else {
        natom as usize
    };
    let mut atoms = Vec::with_capacity(n_atoms);
    for i in 0..n_atoms {
        let row = lines
            .next()
            .ok_or_else(|| format!("cube: missing atom line {i}"))?;
        let p: Vec<&str> = row.split_whitespace().collect();
        if p.len() < 5 {
            return Err(format!("cube: bad atom line {i}"));
        }
        let z_num: u8 = p[0].parse().map_err(|_| "cube: bad atomic number")?;
        let charge: f64 = p[1].parse().map_err(|_| "cube: bad charge")?;
        let x: f64 = p[2].parse().map_err(|_| "cube: bad x")?;
        let y: f64 = p[3].parse().map_err(|_| "cube: bad y")?;
        let zc: f64 = p[4].parse().map_err(|_| "cube: bad z")?;
        let _ = symbol_to_z(crate::elements::z_to_symbol(z_num));
        atoms.push(CubeAtom {
            atomic_number: z_num,
            charge,
            x,
            y,
            z: zc,
        });
    }

    let n_voxels = shape[0] * shape[1] * shape[2];
    let mut values = Vec::with_capacity(n_voxels);
    for row in lines {
        for token in row.split_whitespace() {
            let v: f64 = token.parse().map_err(|_| "cube: bad voxel value")?;
            values.push(v);
            if values.len() == n_voxels {
                break;
            }
        }
        if values.len() == n_voxels {
            break;
        }
    }
    if values.len() != n_voxels {
        return Err(format!(
            "cube: expected {n_voxels} voxels, got {}",
            values.len()
        ));
    }

    // Gaussian cube: x fastest, then y, then z — remap to ScalarField [nx, ny, nz] indexing
    let mut remapped = vec![0.0; n_voxels];
    let [nx, ny, nz] = shape;
    for iz in 0..nz {
        for iy in 0..ny {
            for ix in 0..nx {
                let cube_idx = ix + iy * nx + iz * nx * ny;
                let field_idx = ix + iy * nx + iz * nx * ny;
                remapped[field_idx] = values[cube_idx];
            }
        }
    }

    let corner1 = [
        origin[0] + voxel_vectors[0][0] * nx as f64,
        origin[1] + voxel_vectors[1][1] * ny as f64,
        origin[2] + voxel_vectors[2][2] * nz as f64,
    ];
    let bounds = [
        [
            origin[0].min(corner1[0]),
            origin[1].min(corner1[1]),
            origin[2].min(corner1[2]),
        ],
        [
            origin[0].max(corner1[0]),
            origin[1].max(corner1[1]),
            origin[2].max(corner1[2]),
        ],
    ];

    let field = ScalarField {
        shape,
        values: remapped,
        bounds,
    };

    Ok(CubeVolume {
        title: [title0, title1],
        origin,
        shape,
        voxel_vectors,
        atoms,
        field,
    })
}

pub fn cube_to_scalar_field(cube: &CubeVolume) -> ScalarField {
    cube.field.clone()
}

/// Write a minimal Gaussian cube from an existing scalar field (export stub).
pub fn scalar_field_to_cube(field: &ScalarField, title: &str) -> String {
    let [nx, ny, nz] = field.shape;
    let [[x0, y0, z0], [x1, y1, z1]] = field.bounds;
    let vx = if nx > 1 { (x1 - x0) / (nx - 1) as f64 } else { 1.0 };
    let vy = if ny > 1 { (y1 - y0) / (ny - 1) as f64 } else { 1.0 };
    let vz = if nz > 1 { (z1 - z0) / (nz - 1) as f64 } else { 1.0 };
    let mut out = format!("{title}\nExported from Inertia\n0 {x0:.6} {y0:.6} {z0:.6}\n");
    out.push_str(&format!("{nx} {vx:.6} 0.0 0.0\n"));
    out.push_str(&format!("{ny} 0.0 {vy:.6} 0.0\n"));
    out.push_str(&format!("{nz} 0.0 0.0 {vz:.6}\n"));
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let v = field.values[i + j * nx + k * nx * ny];
                out.push_str(&format!("{v:.6e} "));
                if (i + j * nx + k * nx * ny + 1) % 6 == 0 {
                    out.push('\n');
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_slice::demo_gaussian_field;

    #[test]
    fn export_water_density_cube_example() {
        use crate::scalar_field_to_cube;
        let f = demo_gaussian_field(16);
        let text = scalar_field_to_cube(&f, "Water density (demo)");
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/molecules/water_density.cube"),
            text,
        )
        .expect("write water_density.cube");
    }

    #[test]
    fn roundtrip_cube_export_parse() {
        let field = demo_gaussian_field(8);
        let text = scalar_field_to_cube(&field, "demo");
        let cube = parse_cube(&text).expect("parse exported cube");
        assert_eq!(cube.shape, [8, 8, 8]);
        assert_eq!(cube.field.values.len(), 512);
    }
}
