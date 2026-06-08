//! Build volumetric grids from Gaussian `.fchk` checkpoints.

use crate::elements::vdw_radius;
use crate::fchk::FchkFile;
use crate::fchk_basis::{basis_from_fchk, gto_density_at, gto_mo_at, unpack_density};
use crate::viewer3d::{MoleculeGeometry, ScalarField};

/// Bounding box with padding around a molecule (Bohr).
fn grid_bounds(mol: &MoleculeGeometry, pad: f64) -> [[f64; 3]; 2] {
    let (mut xmin, mut ymin, mut zmin) = (f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let (mut xmax, mut ymax, mut zmax) = (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for a in &mol.atoms {
        xmin = xmin.min(a.x);
        ymin = ymin.min(a.y);
        zmin = zmin.min(a.z);
        xmax = xmax.max(a.x);
        ymax = ymax.max(a.y);
        zmax = zmax.max(a.z);
    }
    if !xmin.is_finite() {
        return [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
    }
    [
        [xmin - pad, ymin - pad, zmin - pad],
        [xmax + pad, ymax + pad, zmax + pad],
    ]
}

fn sample_scalar_grid(
    bounds: [[f64; 3]; 2],
    n: usize,
    mut eval: impl FnMut([f64; 3]) -> f64,
) -> ScalarField {
    let n = n.clamp(8, 64);
    let [x0, y0, z0] = bounds[0];
    let [x1, y1, z1] = bounds[1];
    let nx = n;
    let ny = n;
    let nz = n;
    let mut values = vec![0.0; nx * ny * nz];
    for k in 0..nz {
        let z = z0 + (z1 - z0) * k as f64 / (nz.saturating_sub(1).max(1) as f64);
        for j in 0..ny {
            let y = y0 + (y1 - y0) * j as f64 / (ny.saturating_sub(1).max(1) as f64);
            for i in 0..nx {
                let x = x0 + (x1 - x0) * i as f64 / (nx.saturating_sub(1).max(1) as f64);
                values[i + j * nx + k * nx * ny] = eval([x, y, z]);
            }
        }
    }
    ScalarField {
        shape: [nx, ny, nz],
        values,
        bounds,
    }
}

/// Approximate electron density on a 3D grid by superposing atom-centered Gaussians.
/// Coordinates in the molecule are assumed to be in Bohr (Gaussian convention).
pub fn promolecule_density_field(mol: &MoleculeGeometry, n: usize) -> ScalarField {
    let n = n.clamp(8, 64);
    let pad = 2.5; // Bohr
    let (mut xmin, mut ymin, mut zmin) = (f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let (mut xmax, mut ymax, mut zmax) = (f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for a in &mol.atoms {
        xmin = xmin.min(a.x);
        ymin = ymin.min(a.y);
        zmin = zmin.min(a.z);
        xmax = xmax.max(a.x);
        ymax = ymax.max(a.y);
        zmax = zmax.max(a.z);
    }
    if !xmin.is_finite() {
        return ScalarField {
            shape: [n, n, n],
            values: vec![0.0; n * n * n],
            bounds: [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
        };
    }
    let bounds = [
        [xmin - pad, ymin - pad, zmin - pad],
        [xmax + pad, ymax + pad, zmax + pad],
    ];
    let [x0, y0, z0] = bounds[0];
    let [x1, y1, z1] = bounds[1];
    let nx = n;
    let ny = n;
    let nz = n;
    let mut values = vec![0.0; nx * ny * nz];
    for k in 0..nz {
        let z = z0 + (z1 - z0) * k as f64 / (nz.saturating_sub(1).max(1) as f64);
        for j in 0..ny {
            let y = y0 + (y1 - y0) * j as f64 / (ny.saturating_sub(1).max(1) as f64);
            for i in 0..nx {
                let x = x0 + (x1 - x0) * i as f64 / (nx.saturating_sub(1).max(1) as f64);
                let mut rho = 0.0;
                for a in &mol.atoms {
                    let sigma = vdw_radius(a.element) * 0.35;
                    let dx = x - a.x;
                    let dy = y - a.y;
                    let dz = z - a.z;
                    let r2 = dx * dx + dy * dy + dz * dz;
                    rho += (-r2 / (2.0 * sigma * sigma)).exp();
                }
                values[i + j * nx + k * nx * ny] = rho;
            }
        }
    }
    ScalarField {
        shape: [nx, ny, nz],
        values,
        bounds,
    }
}

/// Build a density grid from an fchk file. Uses GTO evaluation when basis + SCF density
/// are present; otherwise falls back to promolecule approximation.
pub fn fchk_density_field(fchk: &FchkFile, n: usize) -> ScalarField {
    let n = n.clamp(8, 64);
    if let (Some(basis), Some(scf), Some(n_basis)) = (
        basis_from_fchk(fchk),
        fchk.scf_density.as_ref(),
        fchk.n_basis,
    ) {
        if n_basis > 0 && scf.len() >= n_basis * (n_basis + 1) / 2 {
            return gto_density_field(&basis, scf, n_basis, &fchk.geometry, n);
        }
    }
    promolecule_density_field(&fchk.geometry, n)
}

/// Legacy signature — prefer `fchk_density_field(&FchkFile, n)`.
pub fn fchk_density_field_from_geometry(
    geometry: &MoleculeGeometry,
    _n_basis: Option<usize>,
    n: usize,
) -> ScalarField {
    promolecule_density_field(geometry, n)
}

fn gto_density_field(
    basis: &crate::fchk_basis::BasisSet,
    scf: &[f64],
    n_basis: usize,
    mol: &MoleculeGeometry,
    n: usize,
) -> ScalarField {
    let density = unpack_density(scf, n_basis);
    let bounds = grid_bounds(mol, 2.5);
    sample_scalar_grid(bounds, n, |r| gto_density_at(basis, &density, n_basis, r))
}

/// Build a signed MO grid from fchk Alpha MO coefficients (`mo_index` is 0-based).
pub fn fchk_mo_field(fchk: &FchkFile, mo_index: usize, n: usize) -> Result<ScalarField, String> {
    let basis = basis_from_fchk(fchk).ok_or("fchk MO: missing or incomplete basis set")?;
    let coeffs = fchk
        .alpha_mo_coefficients
        .as_ref()
        .ok_or("fchk MO: missing Alpha MO coefficients")?;
    let n_basis = fchk.n_basis.unwrap_or(basis.n_functions());
    let n_mos = coeffs.len() / n_basis.max(1);
    if mo_index >= n_mos {
        return Err(format!(
            "fchk MO: index {mo_index} out of range (n_mos = {n_mos})"
        ));
    }
    let bounds = grid_bounds(&fchk.geometry, 2.5);
    Ok(sample_scalar_grid(bounds, n, |r| {
        gto_mo_at(&basis, coeffs, n_basis, mo_index, r)
    }))
}

fn can_quantum_esp(fchk: &FchkFile) -> bool {
    if basis_from_fchk(fchk).is_none() {
        return false;
    }
    let Some(scf) = fchk.scf_density.as_ref() else {
        return false;
    };
    let Some(n_basis) = fchk.n_basis else {
        return false;
    };
    n_basis > 0 && scf.len() >= n_basis * (n_basis + 1) / 2
}

/// Classical electrostatic potential on a grid (Hartree/e).
/// Uses Mulliken monopoles when present, otherwise bare nuclear charges Z_A/|r-R_A|.
pub fn fchk_classical_esp_field(fchk: &FchkFile, n: usize) -> ScalarField {
    let mol = &fchk.geometry;
    let bounds = grid_bounds(mol, 2.5);
    let charges: Vec<f64> = if let Some(mulliken) = &fchk.mulliken_charges {
        if mulliken.len() == mol.atoms.len() {
            mulliken.clone()
        } else {
            mol.atoms.iter().map(|a| a.element as f64).collect()
        }
    } else {
        mol.atoms.iter().map(|a| a.element as f64).collect()
    };
    let centers: Vec<[f64; 3]> = mol
        .atoms
        .iter()
        .map(|a| [a.x, a.y, a.z])
        .collect();
    const SOFT: f64 = 0.05;
    sample_scalar_grid(bounds, n, |r| {
        let mut v = 0.0;
        for (qi, ci) in charges.iter().zip(centers.iter()) {
            let dx = r[0] - ci[0];
            let dy = r[1] - ci[1];
            let dz = r[2] - ci[2];
            let dist = (dx * dx + dy * dy + dz * dz + SOFT * SOFT).sqrt();
            v += qi / dist;
        }
        v
    })
}

/// Quantum ESP: V(r) = Σ Z_A/|r-R_A| − ∫ ρ(r′)/|r−r′| dr′ (Hartree/e, atomic units).
/// Uses GTO SCF density when basis sections are present; otherwise classical monopole fallback.
pub fn fchk_esp_field(fchk: &FchkFile, n: usize) -> ScalarField {
    if can_quantum_esp(fchk) {
        fchk_quantum_esp_field(fchk, n)
    } else {
        fchk_classical_esp_field(fchk, n)
    }
}

fn grid_voxel_centers(field: &ScalarField) -> (Vec<[f64; 3]>, Vec<f64>, f64) {
    let [nx, ny, nz] = field.shape;
    let [[x0, y0, z0], [x1, y1, z1]] = field.bounds;
    let dx = (x1 - x0) / nx.saturating_sub(1).max(1) as f64;
    let dy = (y1 - y0) / ny.saturating_sub(1).max(1) as f64;
    let dz = (z1 - z0) / nz.saturating_sub(1).max(1) as f64;
    let dv = dx * dy * dz;
    let max_rho = field
        .values
        .iter()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);
    let cutoff = max_rho * 1e-8;
    let mut centers = Vec::new();
    let mut weights = Vec::new();
    for k in 0..nz {
        let z = z0 + (z1 - z0) * k as f64 / nz.saturating_sub(1).max(1) as f64;
        for j in 0..ny {
            let y = y0 + (y1 - y0) * j as f64 / ny.saturating_sub(1).max(1) as f64;
            for i in 0..nx {
                let idx = i + j * nx + k * nx * ny;
                let rho = field.values[idx];
                if rho.abs() < cutoff {
                    continue;
                }
                let x = x0 + (x1 - x0) * i as f64 / nx.saturating_sub(1).max(1) as f64;
                centers.push([x, y, z]);
                weights.push(rho * dv);
            }
        }
    }
    (centers, weights, dv)
}

fn fchk_quantum_esp_field(fchk: &FchkFile, n: usize) -> ScalarField {
    let n = n.clamp(8, 24);
    let rho_field = fchk_density_field(fchk, n);
    let (src_centers, src_weights, _) = grid_voxel_centers(&rho_field);
    let mol = &fchk.geometry;
    let bounds = grid_bounds(mol, 2.5);
    let nuclei: Vec<(f64, [f64; 3])> = mol
        .atoms
        .iter()
        .map(|a| (a.element as f64, [a.x, a.y, a.z]))
        .collect();
    const SOFT: f64 = 0.05;
    sample_scalar_grid(bounds, n, |r| {
        let mut v = 0.0;
        for (z, center) in &nuclei {
            let dx = r[0] - center[0];
            let dy = r[1] - center[1];
            let dz = r[2] - center[2];
            let dist = (dx * dx + dy * dy + dz * dz + SOFT * SOFT).sqrt();
            v += z / dist;
        }
        for (center, w) in src_centers.iter().zip(src_weights.iter()) {
            let dx = r[0] - center[0];
            let dy = r[1] - center[1];
            let dz = r[2] - center[2];
            let dist = (dx * dx + dy * dy + dz * dz + SOFT * SOFT).sqrt();
            v -= w / dist;
        }
        v
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fchk::parse_fchk;
    use crate::viewer3d::{AtomBall, MoleculeGeometry};

    #[test]
    fn promolecule_water_has_peak() {
        let mol = MoleculeGeometry {
            name: "water".into(),
            atoms: vec![
                AtomBall {
                    element: 8,
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    radius: 0.5,
                },
                AtomBall {
                    element: 1,
                    x: 0.757,
                    y: 0.0,
                    z: 0.0,
                    radius: 0.3,
                },
            ],
            bonds: vec![],
        };
        let field = promolecule_density_field(&mol, 16);
        let max = field.values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max > 0.5);
        assert_eq!(field.shape, [16, 16, 16]);
    }

    #[test]
    fn gto_density_from_sto3g_fchk() {
        const STO3G: &str = include_str!("../../../examples/molecules/water_sto3g.fchk");
        let fchk = parse_fchk(STO3G).expect("fchk");
        let field = fchk_density_field(&fchk, 16);
        let max = field.values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max > 0.0);
    }

    #[test]
    fn mo_field_signed() {
        const STO3G: &str = include_str!("../../../examples/molecules/water_sto3g.fchk");
        let fchk = parse_fchk(STO3G).expect("fchk");
        let field = fchk_mo_field(&fchk, 0, 16).expect("mo");
        let max_abs = field
            .values
            .iter()
            .map(|v| v.abs())
            .fold(0.0_f64, f64::max);
        assert!(max_abs > 0.0);
    }

    #[test]
    fn esp_field_finite() {
        const STO3G: &str = include_str!("../../../examples/molecules/water_sto3g.fchk");
        let fchk = parse_fchk(STO3G).expect("fchk");
        let field = fchk_esp_field(&fchk, 16);
        assert!(field.values.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn quantum_esp_differs_from_classical() {
        const STO3G: &str = include_str!("../../../examples/molecules/water_sto3g.fchk");
        let fchk = parse_fchk(STO3G).expect("fchk");
        assert!(can_quantum_esp(&fchk));
        let quantum = fchk_quantum_esp_field(&fchk, 12);
        let classical = fchk_classical_esp_field(&fchk, 12);
        let max_diff = quantum
            .values
            .iter()
            .zip(classical.values.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max);
        assert!(max_diff > 0.01, "electron Hartree term should shift ESP");
    }
}
