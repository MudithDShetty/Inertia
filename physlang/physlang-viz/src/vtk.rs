//! VTK legacy export for scalar fields (ImageData / STRUCTURED_POINTS).

use crate::viewer3d::ScalarField;

/// Write ASCII VTK 3.0 structured points for ParaView / VisIt.
pub fn scalar_field_to_vtk(field: &ScalarField, title: &str) -> String {
    let [nx, ny, nz] = field.shape;
    let [[x0, y0, z0], [x1, y1, z1]] = field.bounds;
    let dx = if nx > 1 { (x1 - x0) / (nx - 1) as f64 } else { 1.0 };
    let dy = if ny > 1 { (y1 - y0) / (ny - 1) as f64 } else { 1.0 };
    let dz = if nz > 1 { (z1 - z0) / (nz - 1) as f64 } else { 1.0 };
    let n = nx * ny * nz;

    let mut out = String::new();
    out.push_str("# vtk DataFile Version 3.0\n");
    out.push_str(&format!("{title}\n"));
    out.push_str("ASCII\n");
    out.push_str("DATASET STRUCTURED_POINTS\n");
    out.push_str(&format!("DIMENSIONS {nx} {ny} {nz}\n"));
    out.push_str(&format!("ORIGIN {x0:.8e} {y0:.8e} {z0:.8e}\n"));
    out.push_str(&format!("SPACING {dx:.8e} {dy:.8e} {dz:.8e}\n"));
    out.push_str(&format!("POINT_DATA {n}\n"));
    out.push_str("SCALARS scalar float 1\n");
    out.push_str("LOOKUP_TABLE default\n");
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let v = field.values[i + j * nx + k * nx * ny];
                out.push_str(&format!("{v:.8e}\n"));
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
    fn vtk_export_has_header_and_values() {
        let f = demo_gaussian_field(4);
        let vtk = scalar_field_to_vtk(&f, "demo");
        assert!(vtk.contains("STRUCTURED_POINTS"));
        assert!(vtk.contains("POINT_DATA 64"));
    }
}
