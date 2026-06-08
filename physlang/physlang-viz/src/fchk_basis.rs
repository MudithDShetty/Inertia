//! Gaussian basis set parsed from `.fchk` for GTO density evaluation.

use crate::fchk::FchkFile;
use crate::viewer3d::MoleculeGeometry;

/// One contracted Cartesian Gaussian basis function.
#[derive(Debug, Clone)]
pub struct BasisFunction {
    pub center: [f64; 3],
    /// Cartesian angular part: (lx, ly, lz) with lx+ly+lz = l.
    pub ang: [u8; 3],
    pub primitives: Vec<(f64, f64)>, // (coefficient, exponent)
}

#[derive(Debug, Clone)]
pub struct BasisSet {
    pub functions: Vec<BasisFunction>,
}

impl BasisSet {
    pub fn n_functions(&self) -> usize {
        self.functions.len()
    }

    /// Evaluate basis function μ at point r.
    pub fn eval(&self, mu: usize, r: [f64; 3]) -> f64 {
        let bf = &self.functions[mu];
        let [lx, ly, lz] = bf.ang;
        let dx = r[0] - bf.center[0];
        let dy = r[1] - bf.center[1];
        let dz = r[2] - bf.center[2];
        let mut val = 0.0;
        for &(c, alpha) in &bf.primitives {
            let gauss = (-alpha * (dx * dx + dy * dy + dz * dz)).exp();
            val += c * cartesian_polynomial(dx, dy, dz, lx, ly, lz) * gauss;
        }
        val
    }
}

fn cartesian_polynomial(dx: f64, dy: f64, dz: f64, lx: u8, ly: u8, lz: u8) -> f64 {
    let mut v = 1.0;
    for _ in 0..lx {
        v *= dx;
    }
    for _ in 0..ly {
        v *= dy;
    }
    for _ in 0..lz {
        v *= dz;
    }
    v
}

/// Expand shell angular momentum into Cartesian components.
fn cartesian_angular_parts(l: i64) -> Vec<[u8; 3]> {
    match l {
        0 => vec![[0, 0, 0]],
        1 => vec![[1, 0, 0], [0, 1, 0], [0, 0, 1]],
        2 => {
            let mut parts = Vec::new();
            for lx in 0..=2 {
                for ly in 0..=(2 - lx) {
                    let lz = 2 - lx - ly;
                    parts.push([lx as u8, ly as u8, lz as u8]);
                }
            }
            parts
        }
        3 => {
            let mut parts = Vec::new();
            for lx in 0..=3 {
                for ly in 0..=(3 - lx) {
                    let lz = 3 - lx - ly;
                    parts.push([lx as u8, ly as u8, lz as u8]);
                }
            }
            parts
        }
        _ => vec![],
    }
}

/// Build a basis set from parsed fchk sections; returns None if data is incomplete.
pub fn basis_from_fchk(fchk: &FchkFile) -> Option<BasisSet> {
    let shell_types = fchk.shell_types.as_ref()?;
    let n_prim = fchk.primitives_per_shell.as_ref()?;
    let shell_atoms = fchk.shell_to_atom.as_ref()?;
    let exponents = fchk.primitive_exponents.as_ref()?;
    let coeffs = fchk.contraction_coefficients.as_ref()?;
    if shell_types.len() != n_prim.len() || shell_types.len() != shell_atoms.len() {
        return None;
    }

    let coords = atom_coords(&fchk.geometry);
    let mut functions = Vec::new();
    let mut prim_offset = 0usize;
    let mut coeff_offset = 0usize;

    for (si, &shell_type) in shell_types.iter().enumerate() {
        let n_p = n_prim[si] as usize;
        if n_p == 0 {
            continue;
        }
        let atom_idx = shell_atoms[si].saturating_sub(1) as usize;
        let center = coords.get(atom_idx).copied().unwrap_or([0.0, 0.0, 0.0]);

        let exps: Vec<f64> = exponents
            .iter()
            .skip(prim_offset)
            .take(n_p)
            .copied()
            .collect();
        prim_offset += n_p;

        if shell_type == -1 {
            // SP shell: s + 3p with separate contraction blocks.
            let s_coeffs: Vec<f64> = coeffs
                .iter()
                .skip(coeff_offset)
                .take(n_p)
                .copied()
                .collect();
            coeff_offset += n_p;
            let p_coeffs: Vec<f64> = coeffs
                .iter()
                .skip(coeff_offset)
                .take(n_p)
                .copied()
                .collect();
            coeff_offset += n_p;

            let s_prims: Vec<(f64, f64)> = s_coeffs.into_iter().zip(exps.iter().copied()).collect();
            functions.push(BasisFunction {
                center,
                ang: [0, 0, 0],
                primitives: s_prims,
            });
            for ang in cartesian_angular_parts(1) {
                let p_prims: Vec<(f64, f64)> =
                    p_coeffs.iter().copied().zip(exps.iter().copied()).collect();
                functions.push(BasisFunction {
                    center,
                    ang,
                    primitives: p_prims,
                });
            }
        } else if shell_type >= 0 {
            let l = shell_type;
            let n_coeff = n_p;
            let shell_coeffs: Vec<f64> = coeffs
                .iter()
                .skip(coeff_offset)
                .take(n_coeff)
                .copied()
                .collect();
            coeff_offset += n_coeff;

            let prims: Vec<(f64, f64)> = shell_coeffs.into_iter().zip(exps.iter().copied()).collect();
            for ang in cartesian_angular_parts(l) {
                functions.push(BasisFunction {
                    center,
                    ang,
                    primitives: prims.clone(),
                });
            }
        }
    }

    if functions.is_empty() {
        return None;
    }
    Some(BasisSet { functions })
}

fn atom_coords(mol: &MoleculeGeometry) -> Vec<[f64; 3]> {
    mol.atoms
        .iter()
        .map(|a| [a.x, a.y, a.z])
        .collect()
}

/// Unpack lower-triangle SCF density to symmetric matrix.
pub fn unpack_density(scf: &[f64], n: usize) -> Vec<f64> {
    let expected = n * (n + 1) / 2;
    if scf.len() < expected {
        return vec![0.0; n * n];
    }
    let mut p = vec![0.0; n * n];
    let mut k = 0usize;
    for i in 0..n {
        for j in 0..=i {
            let v = scf[k];
            p[i * n + j] = v;
            p[j * n + i] = v;
            k += 1;
        }
    }
    p
}

/// Electron density ρ(r) = Σ_μν P_μν φ_μ(r) φ_ν(r).
pub fn gto_density_at(basis: &BasisSet, density: &[f64], n: usize, r: [f64; 3]) -> f64 {
    let nf = basis.n_functions().min(n);
    let mut rho = 0.0;
    for mu in 0..nf {
        let phi_mu = basis.eval(mu, r);
        for nu in 0..nf {
            let p = density[mu * n + nu];
            if p.abs() < 1e-20 {
                continue;
            }
            rho += p * phi_mu * basis.eval(nu, r);
        }
    }
    rho.max(0.0)
}

/// MO wavefunction ψ_i(r) = Σ_μ C_μi φ_μ(r); `mo_index` is 0-based.
pub fn gto_mo_at(
    basis: &BasisSet,
    mo_coeffs: &[f64],
    n_basis: usize,
    mo_index: usize,
    r: [f64; 3],
) -> f64 {
    let nf = basis.n_functions().min(n_basis);
    let offset = mo_index.saturating_mul(n_basis);
    if offset + nf > mo_coeffs.len() {
        return 0.0;
    }
    let mut psi = 0.0;
    for mu in 0..nf {
        psi += mo_coeffs[offset + mu] * basis.eval(mu, r);
    }
    psi
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fchk::parse_fchk;
    use crate::viewer3d::{AtomBall, MoleculeGeometry};

    const STO3G_WATER_FCHK: &str = include_str!("../../../examples/molecules/water_sto3g.fchk");

    #[test]
    fn d_shell_has_six_cartesian() {
        assert_eq!(cartesian_angular_parts(2).len(), 6);
    }

    #[test]
    fn f_shell_has_ten_cartesian() {
        assert_eq!(cartesian_angular_parts(3).len(), 10);
    }

    #[test]
    fn parses_sto3g_basis() {
        let fchk = parse_fchk(STO3G_WATER_FCHK).expect("fchk");
        let basis = basis_from_fchk(&fchk).expect("basis");
        assert_eq!(basis.n_functions(), 7);
    }

    #[test]
    fn gto_density_positive_at_nucleus() {
        let fchk = parse_fchk(STO3G_WATER_FCHK).expect("fchk");
        let basis = basis_from_fchk(&fchk).expect("basis");
        let n = fchk.n_basis.unwrap_or(basis.n_functions());
        let p = unpack_density(fchk.scf_density.as_ref().unwrap(), n);
        let o = &fchk.geometry.atoms[0];
        let rho = gto_density_at(&basis, &p, n, [o.x, o.y, o.z]);
        assert!(rho > 0.0);
    }

    #[test]
    fn unpack_symmetric() {
        let tri = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let p = unpack_density(&tri, 3);
        assert!((p[0] - 1.0).abs() < 1e-12);
        assert!((p[4] - 3.0).abs() < 1e-12);
        assert!((p[3] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn gto_mo_identity_coeffs() {
        let fchk = parse_fchk(STO3G_WATER_FCHK).expect("fchk");
        let basis = basis_from_fchk(&fchk).expect("basis");
        let n = fchk.n_basis.unwrap();
        let coeffs = fchk.alpha_mo_coefficients.as_ref().expect("mos");
        let o = &fchk.geometry.atoms[0];
        let psi0 = gto_mo_at(&basis, coeffs, n, 0, [o.x, o.y, o.z]);
        assert!(psi0.abs() > 0.0);
    }
}
