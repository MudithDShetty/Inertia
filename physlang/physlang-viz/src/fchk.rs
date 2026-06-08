//! Gaussian formatted checkpoint (`.fchk`) — minimal section parser.

use crate::elements::z_to_symbol;
use crate::viewer3d::{infer_bonds, AtomBall, MoleculeGeometry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FchkFile {
    pub title: String,
    pub geometry: MoleculeGeometry,
    pub vibrational_frequencies_cm1: Vec<f64>,
    pub has_density: bool,
    pub has_mos: bool,
    pub n_basis: Option<usize>,
    /// Alpha-spin electrons (RHF closed shell → occupied spatial MO count).
    pub n_alpha_electrons: Option<usize>,
    /// Hartree; one per spatial MO (lowest first).
    pub orbital_energies: Option<Vec<f64>>,
    /// Flat `n_mos * n_basis` — all coefficients for MO 1, then MO 2, …
    pub alpha_mo_coefficients: Option<Vec<f64>>,
    /// Mulliken charges per atom (elementary charge); used for ESP monopole stub.
    pub mulliken_charges: Option<Vec<f64>>,
    /// Lower-triangle packed SCF density matrix (length n_basis*(n_basis+1)/2).
    pub scf_density: Option<Vec<f64>>,
    /// Basis-set sections for GTO density evaluation.
    pub shell_types: Option<Vec<i64>>,
    pub primitives_per_shell: Option<Vec<i64>>,
    pub shell_to_atom: Option<Vec<i64>>,
    pub primitive_exponents: Option<Vec<f64>>,
    pub contraction_coefficients: Option<Vec<f64>>,
}

pub fn parse_fchk(source: &str) -> Result<FchkFile, String> {
    let sections = parse_sections(source)?;
    let n_atoms = sections
        .get_int("Number of atoms")
        .ok_or("fchk: missing Number of atoms")? as usize;

    let atomic_numbers = sections
        .get_int_vec("Atomic numbers")
        .ok_or("fchk: missing Atomic numbers")?;
    if atomic_numbers.len() != n_atoms {
        return Err(format!(
            "fchk: atomic numbers len {} != n_atoms {n_atoms}",
            atomic_numbers.len()
        ));
    }

    let coords = sections
        .get_float_vec("Current cartesian coordinates")
        .or_else(|| sections.get_float_vec("Cartesian coordinates"))
        .ok_or("fchk: missing coordinates")?;
    if coords.len() != n_atoms * 3 {
        return Err(format!(
            "fchk: coordinates len {} != {}",
            coords.len(),
            n_atoms * 3
        ));
    }

    let mut atoms = Vec::with_capacity(n_atoms);
    for i in 0..n_atoms {
        let z = atomic_numbers[i] as u8;
        atoms.push(AtomBall {
            element: z,
            x: coords[i * 3],
            y: coords[i * 3 + 1],
            z: coords[i * 3 + 2],
            radius: 0.5,
        });
    }

    let title = sections
        .strings
        .get("Route")
        .map(|v| v.join(" "))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "fchk".into());

    let mut mol = MoleculeGeometry {
        name: title.clone(),
        atoms,
        bonds: vec![],
    };
    infer_bonds(&mut mol);

    let freqs = sections
        .get_float_vec("Vibrational frequencies")
        .unwrap_or_default();
    let scf_density = sections.get_float_vec("Total SCF Density");
    let has_density = scf_density.is_some();
    let alpha_mo_coefficients = sections.get_float_vec("Alpha MO coefficients");
    let has_mos = alpha_mo_coefficients.is_some();
    let n_basis = sections
        .get_int("Number of basis functions")
        .map(|n| n as usize);
    let n_alpha_electrons = sections
        .get_int("Number of alpha electrons")
        .map(|n| n as usize);
    let orbital_energies = sections
        .get_float_vec("Alpha Orbital Energies")
        .or_else(|| sections.get_float_vec("Orbital Energies"));
    let mulliken_charges = sections.get_float_vec("Mulliken charges");

    let shell_types = sections.get_int_vec("Shell types");
    let primitives_per_shell = sections.get_int_vec("Number of primitives per shell");
    let shell_to_atom = sections
        .get_int_vec("Shell to atom map")
        .or_else(|| sections.get_int_vec("Shell to atom mapping"));
    let primitive_exponents = sections.get_float_vec("Primitive exponents");
    let contraction_coefficients = sections
        .get_float_vec("Contraction coefficients")
        .or_else(|| sections.get_float_vec("(P) Contraction coefficients"));

    Ok(FchkFile {
        title,
        geometry: mol,
        vibrational_frequencies_cm1: freqs,
        has_density,
        has_mos,
        n_basis,
        n_alpha_electrons,
        orbital_energies,
        alpha_mo_coefficients,
        mulliken_charges,
        scf_density,
        shell_types,
        primitives_per_shell,
        shell_to_atom,
        primitive_exponents,
        contraction_coefficients,
    })
}

pub fn parse_fchk_geometry(source: &str) -> Result<MoleculeGeometry, String> {
    Ok(parse_fchk(source)?.geometry)
}

impl FchkFile {
    /// Number of spatial MOs implied by coefficient vector length.
    pub fn n_mos(&self) -> Option<usize> {
        let coeffs = self.alpha_mo_coefficients.as_ref()?;
        let n_basis = self.n_basis?;
        if n_basis == 0 {
            return None;
        }
        Some(coeffs.len() / n_basis)
    }

    /// 0-based index of highest occupied spatial MO (RHF closed shell).
    pub fn homo_index(&self) -> Option<usize> {
        let n_alpha = self.n_alpha_electrons?;
        Some(n_alpha.saturating_sub(1))
    }

    /// 0-based index of lowest unoccupied spatial MO.
    pub fn lumo_index(&self) -> Option<usize> {
        self.homo_index().map(|h| h + 1)
    }
}

#[derive(Debug, Default)]
struct SectionMap {
    ints: std::collections::HashMap<String, Vec<i64>>,
    floats: std::collections::HashMap<String, Vec<f64>>,
    strings: std::collections::HashMap<String, Vec<String>>,
}

impl SectionMap {
    fn get_int(&self, key: &str) -> Option<i64> {
        self.ints.get(key).and_then(|v| v.first().copied())
    }

    fn get_int_vec(&self, key: &str) -> Option<Vec<i64>> {
        self.ints.get(key).cloned()
    }

    fn get_float_vec(&self, key: &str) -> Option<Vec<f64>> {
        self.floats.get(key).cloned()
    }

    fn contains_key(&self, key: &str) -> bool {
        self.ints.contains_key(key)
            || self.floats.contains_key(key)
            || self.strings.contains_key(key)
    }
}

fn parse_sections(source: &str) -> Result<SectionMap, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut map = SectionMap::default();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim_end();
        if let Some((name, kind, count, inline)) = parse_section_header(line) {
            i += 1;
            let mut values: Vec<String> = if inline {
                vec![count.to_string()]
            } else {
                let mut values: Vec<String> = Vec::new();
                while i < lines.len() && values.len() < count {
                    values.extend(lines[i].split_whitespace().map(str::to_string));
                    i += 1;
                }
                values
            };
            if !inline {
                values.truncate(count);
            }
            match kind {
                'I' => {
                    let ints: Result<Vec<i64>, _> =
                        values.iter().map(|s| s.parse()).collect();
                    map.ints.insert(name, ints.map_err(|e| e.to_string())?);
                }
                'R' => {
                    let floats: Result<Vec<f64>, _> =
                        values.iter().map(|s| s.parse()).collect();
                    map.floats.insert(name, floats.map_err(|e| e.to_string())?);
                }
                'C' => {
                    map.strings.insert(name, values);
                }
                _ => {}
            }
            continue;
        }
        i += 1;
    }
    Ok(map)
}

fn parse_section_header(line: &str) -> Option<(String, char, usize, bool)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let kind_idx = parts
        .iter()
        .position(|p| matches!(*p, "I" | "R" | "C"))?;
    let kind = parts[kind_idx].chars().next()?;
    let name = parts[..kind_idx].join(" ");
    if name.is_empty() {
        return None;
    }

    // `N= 9` or `N=9` → read 9 values from following lines; bare `I 3` → scalar inline.
    if let Some(n_eq_idx) = parts.iter().position(|p| p.starts_with("N=")) {
        let token = parts[n_eq_idx];
        let count: usize = if token.len() > 2 {
            token[2..].parse().ok()?
        } else {
            parts.get(n_eq_idx + 1)?.parse().ok()?
        };
        return Some((name, kind, count, false));
    }
    if let Some(n_idx) = parts.iter().position(|p| *p == "N") {
        let count: usize = parts.get(n_idx + 1)?.parse().ok()?;
        return Some((name, kind, count, false));
    }
    let inline: usize = parts.last()?.parse().ok()?;
    Some((name, kind, inline, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINI_FCHK: &str = r#"Water sto-3g
    1    1    0
    3
    8    1    1
    0    1
Number of atoms                            I                3
Number of electrons                         I                10
Atomic numbers                             I   N=           3
           8           1           1
Current cartesian coordinates              R   N=           9
  0.000000   0.000000   0.117000  -0.757000   0.000000  -0.468000
   0.757000   0.000000  -0.468000
Total SCF Density                          R   N=           6
   0.0   0.0   0.0   0.0   0.0   0.0
"#;

    #[test]
    fn parses_mini_fchk() {
        let f = parse_fchk(MINI_FCHK).expect("fchk");
        assert_eq!(f.geometry.atoms.len(), 3);
        assert_eq!(f.geometry.atoms[0].element, 8);
        assert!(f.has_density);
        assert_eq!(z_to_symbol(f.geometry.atoms[0].element), "O");
    }

    #[test]
    fn parses_mo_sections() {
        const STO3G: &str = include_str!("../../../examples/molecules/water_sto3g.fchk");
        let f = parse_fchk(STO3G).expect("fchk");
        assert!(f.has_mos);
        assert_eq!(f.n_mos(), Some(7));
        assert_eq!(f.homo_index(), Some(4));
        assert_eq!(f.lumo_index(), Some(5));
    }
}
