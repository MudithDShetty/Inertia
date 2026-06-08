//! Gaussian output (.log) parser — geometries, SCF energies, freq stub.

use crate::elements::{symbol_to_z, z_to_symbol};
use crate::viewer3d::{infer_bonds, AtomBall, MoleculeGeometry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaussianLogResult {
    pub title: String,
    pub scf_energies_hartree: Vec<f64>,
    pub final_energy_hartree: Option<f64>,
    pub geometry: Option<MoleculeGeometry>,
    pub n_frequencies: usize,
    pub zero_point_correction_hartree: Option<f64>,
    pub thermal_correction_hartree: Option<f64>,
}

pub fn parse_gaussian_log(source: &str) -> Result<GaussianLogResult, String> {
    let title = extract_title(source);
    let scf_energies = extract_scf_energies(source);
    let final_energy = scf_energies.last().copied();
    let geometry = extract_last_standard_orientation(source)?;
    let (n_frequencies, zpe, thermal) = extract_thermochemistry_stub(source);

    if geometry.is_none() && scf_energies.is_empty() {
        return Err("log: no standard orientation or SCF energies found".into());
    }

    Ok(GaussianLogResult {
        title,
        scf_energies_hartree: scf_energies,
        final_energy_hartree: final_energy,
        geometry,
        n_frequencies,
        zero_point_correction_hartree: zpe,
        thermal_correction_hartree: thermal,
    })
}

fn extract_title(source: &str) -> String {
    for line in source.lines() {
        let t = line.trim();
        if !t.is_empty() && !t.starts_with('#') && !t.starts_with(" Entering") {
            return t.to_string();
        }
    }
    "Gaussian log".into()
}

fn extract_scf_energies(source: &str) -> Vec<f64> {
    let mut energies = Vec::new();
    for line in source.lines() {
        if line.contains("SCF Done:") {
            if let Some(e) = parse_scf_energy_line(line) {
                energies.push(e);
            }
        }
    }
    energies
}

fn parse_scf_energy_line(line: &str) -> Option<f64> {
    let after = line.split('=').nth(1)?;
    for token in after.split_whitespace() {
        if let Ok(v) = token.parse::<f64>() {
            return Some(v);
        }
    }
    None
}

fn extract_last_standard_orientation(source: &str) -> Result<Option<MoleculeGeometry>, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut last_atoms: Option<Vec<AtomBall>> = None;
    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("Standard orientation:") {
            i += 1;
            let mut atoms = Vec::new();
            while i < lines.len() {
                let trimmed = lines[i].trim();
                if trimmed.is_empty() {
                    i += 1;
                    continue;
                }
                if trimmed.starts_with('-')
                    || trimmed.starts_with("Center")
                    || trimmed.starts_with("Number")
                {
                    i += 1;
                    continue;
                }
                if let Some(atom) = parse_orientation_line(lines[i]) {
                    atoms.push(atom);
                    i += 1;
                    continue;
                }
                break;
            }
            if !atoms.is_empty() {
                last_atoms = Some(atoms);
            }
            continue;
        }
        i += 1;
    }

    Ok(last_atoms.map(|atoms| {
        let mut mol = MoleculeGeometry {
            name: "optimized geometry".into(),
            atoms,
            bonds: vec![],
        };
        infer_bonds(&mut mol);
        mol
    }))
}

fn parse_orientation_line(line: &str) -> Option<AtomBall> {
    let line = line.trim_end_matches('\r');
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }
    let z_num: u8 = parts[1].parse().ok()?;
    let x: f64 = parts[3].parse().ok()?;
    let y: f64 = parts[4].parse().ok()?;
    let zc: f64 = parts[5].parse().ok()?;
    Some(AtomBall {
        element: z_num,
        x,
        y,
        z: zc,
        radius: 0.5,
    })
}

fn extract_thermochemistry_stub(source: &str) -> (usize, Option<f64>, Option<f64>) {
    let mut n_freq = 0usize;
    let mut zpe = None;
    let mut thermal = None;
    for line in source.lines() {
        if line.contains("Frequencies --") {
            n_freq += line.matches("--").count().saturating_sub(0);
            n_freq += line.split_whitespace().filter(|t| t.parse::<f64>().is_ok()).count();
        }
        if line.contains("Zero-point correction=") {
            zpe = parse_trailing_float(line);
        }
        if line.contains("Thermal correction to Energy=") {
            thermal = parse_trailing_float(line);
        }
    }
    (n_freq, zpe, thermal)
}

fn parse_trailing_float(line: &str) -> Option<f64> {
    line.split('=').nth(1)?.trim().split_whitespace().next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER_LOG: &str = r#" Water SP

 # HF/6-31G(d) SP

 SCF Done:  E(RHF) =  -76.010000     A.U. after    6 cycles
                         Standard orientation:                         
 ---------------------------------------------------------------------
 Center     Atomic      Atomic             Coordinates (Angstroms)
 Number     Number       Type             X           Y           Z
 ---------------------------------------------------------------------
      1          8           0        0.000000    0.000000    0.117000
      2          1           0       -0.757000    0.000000   -0.468000
      3          1           0        0.757000    0.000000   -0.468000
 ---------------------------------------------------------------------
"#;

    #[test]
    fn parses_log_geometry_and_energy() {
        let r = parse_gaussian_log(WATER_LOG).expect("parse log");
        assert!(r.geometry.is_some(), "expected geometry, scf={:?}", r.scf_energies_hartree);
        assert_eq!(r.geometry.as_ref().unwrap().atoms.len(), 3);
        assert!(r.final_energy_hartree.is_some());
        assert!((r.final_energy_hartree.unwrap() + 76.01).abs() < 1e-4);
    }
}
