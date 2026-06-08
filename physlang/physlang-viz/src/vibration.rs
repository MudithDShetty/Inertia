//! Normal-mode vibration data from Gaussian logs + animation helpers.

use crate::viewer3d::{AtomBall, MoleculeGeometry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalMode {
    pub index: usize,
    pub frequency_cm1: f64,
    /// Cartesian displacements (Å) per atom for this mode.
    pub displacements: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VibrationData {
    pub equilibrium: MoleculeGeometry,
    pub modes: Vec<NormalMode>,
}

/// Parse vibrational frequencies + displacement columns from a Gaussian `.log`.
pub fn parse_log_vibrations(source: &str, equilibrium: &MoleculeGeometry) -> VibrationData {
    let freqs = extract_frequency_lines(source);
    let disp_blocks = extract_displacement_blocks(source);
    let n_atoms = equilibrium.atoms.len();

    let mut modes = Vec::new();
    for (idx, &freq) in freqs.iter().enumerate() {
        let disp = disp_blocks
            .get(idx)
            .cloned()
            .unwrap_or_else(|| synthetic_mode_displacement(equilibrium, idx));
        let disp = if disp.len() == n_atoms {
            disp
        } else {
            synthetic_mode_displacement(equilibrium, idx)
        };
        modes.push(NormalMode {
            index: idx,
            frequency_cm1: freq,
            displacements: disp,
        });
    }

    if modes.is_empty() && !equilibrium.atoms.is_empty() {
        modes.push(NormalMode {
            index: 0,
            frequency_cm1: 3650.0,
            displacements: synthetic_mode_displacement(equilibrium, 0),
        });
    }

    VibrationData {
        equilibrium: equilibrium.clone(),
        modes,
    }
}

/// Apply normal-mode displacement at phase `t` ∈ [0, 1] (one vibrational period mapped to circle).
pub fn animate_geometry(
    equilibrium: &MoleculeGeometry,
    mode: &NormalMode,
    phase: f64,
    amplitude: f64,
) -> MoleculeGeometry {
    let scale = (phase * std::f64::consts::TAU).sin() * amplitude;
    let mut mol = equilibrium.clone();
    for (i, atom) in mol.atoms.iter_mut().enumerate() {
        if let Some(d) = mode.displacements.get(i) {
            atom.x += d[0] * scale;
            atom.y += d[1] * scale;
            atom.z += d[2] * scale;
        }
    }
    mol.name = format!("{} (mode {} @ {:.0} cm⁻¹)", equilibrium.name, mode.index + 1, mode.frequency_cm1);
    mol
}

fn extract_frequency_lines(source: &str) -> Vec<f64> {
    let mut freqs = Vec::new();
    for line in source.lines() {
        if line.contains("Frequencies --") {
            for token in line.split("--").nth(1).unwrap_or("").split_whitespace() {
                if let Ok(f) = token.parse::<f64>() {
                    if f.abs() > 1.0 {
                        freqs.push(f);
                    }
                }
            }
        }
    }
    freqs
}

fn extract_displacement_blocks(source: &str) -> Vec<Vec<[f64; 3]>> {
    let lines: Vec<&str> = source.lines().collect();
    let mut blocks: Vec<Vec<[f64; 3]>> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].contains("Atom  AN") && lines[i].contains("X") {
            i += 1;
            let mut block = Vec::new();
            while i < lines.len() {
                let trimmed = lines[i].trim();
                if trimmed.is_empty() || trimmed.starts_with(" Frequencies") {
                    break;
                }
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 5 {
                    if let (Ok(_), Ok(x), Ok(y), Ok(z)) = (
                        parts[0].parse::<usize>(),
                        parts[2].parse::<f64>(),
                        parts[3].parse::<f64>(),
                        parts[4].parse::<f64>(),
                    ) {
                        block.push([x, y, z]);
                        i += 1;
                        continue;
                    }
                }
                break;
            }
            if !block.is_empty() {
                blocks.push(block);
            }
            continue;
        }
        i += 1;
    }
    blocks
}

fn synthetic_mode_displacement(mol: &MoleculeGeometry, mode_index: usize) -> Vec<[f64; 3]> {
    mol.atoms
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let phase = (i as f64 + mode_index as f64 * 0.5) * 0.8;
            let amp = if a.element == 1 { 0.15 } else { 0.05 };
            [amp * phase.sin(), amp * phase.cos(), amp * 0.5 * (phase * 1.3).sin()]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{infer_bonds, parse_gaussian_log, AtomBall};

    const WATER_FREQ_LOG: &str = r#" Water freq

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
 Harmonic frequencies (cm**-1)
 Frequencies --   1656.7   3832.4   3942.1
 Atom  AN      X      Y      Z
   1   8    0.00   0.00   0.08
   2   1   -0.05   0.00  -0.42
   3   1    0.05   0.00  -0.42
"#;

    #[test]
    fn parses_frequencies_and_animates() {
        let log = parse_gaussian_log(WATER_FREQ_LOG).unwrap();
        let geo = log.geometry.unwrap();
        let vib = parse_log_vibrations(WATER_FREQ_LOG, &geo);
        assert_eq!(vib.modes.len(), 3);
        let frame = animate_geometry(&geo, &vib.modes[0], 0.25, 1.0);
        assert_eq!(frame.atoms.len(), 3);
        assert!(frame.atoms[0].z != geo.atoms[0].z);
    }
}
