//! Gaussian input (.gjf / .com) parser — GaussView-compatible structure import.

use crate::elements::{normalize_symbol, symbol_to_z, vdw_radius};
use crate::viewer3d::{infer_bonds, AtomBall, MoleculeGeometry};
use serde::{Deserialize, Serialize};

/// GaussView-style display mode for molecular rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MolRenderStyle {
    #[default]
    BallAndStick,
    Wireframe,
    SpaceFill,
    Stick,
}

impl MolRenderStyle {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "wireframe" | "wire" => Self::Wireframe,
            "spacefill" | "space_fill" | "cpk" => Self::SpaceFill,
            "stick" | "sticks" => Self::Stick,
            _ => Self::BallAndStick,
        }
    }
}

/// Parsed Gaussian job input (structure + route metadata).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaussianInput {
    pub route: String,
    pub title: String,
    pub charge: i32,
    pub multiplicity: u32,
    pub geometry: MoleculeGeometry,
    pub coordinate_type: CoordinateType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateType {
    Cartesian,
    ZMatrix,
}

pub fn parse_gjf(source: &str) -> Result<GaussianInput, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0usize;

    // Route section (# lines, optional \ continuation)
    let mut route_parts = Vec::new();
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        if line.starts_with('#') {
            let mut part = line.trim_end_matches('\\').trim().to_string();
            route_parts.push(part.clone());
            while lines[i].trim().ends_with('\\') && i + 1 < lines.len() {
                i += 1;
                part = format!(
                    "{} {}",
                    part,
                    lines[i].trim().trim_start_matches('#').trim()
                );
                if let Some(last) = route_parts.last_mut() {
                    *last = part.clone();
                }
            }
            i += 1;
        } else {
            break;
        }
    }
    let route = route_parts.join(" ");

    // Skip blank lines
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }

    // Title (until charge/mult or blank block end)
    let mut title_lines = Vec::new();
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            break;
        }
        if line.starts_with('#') || line.starts_with("--") {
            break;
        }
        if parse_charge_mult(line).is_some() {
            break;
        }
        title_lines.push(line.to_string());
        i += 1;
    }
    let title = if title_lines.is_empty() {
        "Gaussian job".into()
    } else {
        title_lines.join(" ")
    };

    // Skip blanks before charge/mult
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }

    let (charge, mult) = lines
        .get(i)
        .and_then(|l| parse_charge_mult(l.trim()))
        .ok_or("gjf: missing charge/multiplicity line")?;
    i += 1;

    // Coordinate block
    let mut coord_lines = Vec::new();
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            break;
        }
        if line.starts_with("--") || line.starts_with('#') {
            break;
        }
        coord_lines.push(line.to_string());
        i += 1;
    }

    if coord_lines.is_empty() {
        return Err("gjf: no coordinate lines".into());
    }

    let (atoms, coord_type) = if looks_cartesian(&coord_lines[0]) {
        (parse_cartesian(&coord_lines)?, CoordinateType::Cartesian)
    } else {
        (zmatrix_to_cartesian(&coord_lines)?, CoordinateType::ZMatrix)
    };

    let mut mol = MoleculeGeometry {
        name: title.clone(),
        atoms,
        bonds: vec![],
    };
    infer_bonds(&mut mol);

    Ok(GaussianInput {
        route,
        title,
        charge,
        multiplicity: mult,
        geometry: mol,
        coordinate_type: coord_type,
    })
}

pub fn parse_gjf_geometry(source: &str) -> Result<MoleculeGeometry, String> {
    Ok(parse_gjf(source)?.geometry)
}

fn parse_charge_mult(line: &str) -> Option<(i32, u32)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let charge: i32 = parts[0].parse().ok()?;
    let mult: u32 = parts[1].parse().ok()?;
    Some((charge, mult))
}

fn looks_cartesian(line: &str) -> bool {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return false;
    }
    parts[1].parse::<f64>().is_ok()
        && parts[2].parse::<f64>().is_ok()
        && parts[3].parse::<f64>().is_ok()
}

fn parse_cartesian(lines: &[String]) -> Result<Vec<AtomBall>, String> {
    let mut atoms = Vec::with_capacity(lines.len());
    for (idx, line) in lines.iter().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(format!("gjf: cartesian line {} expected element x y z", idx + 1));
        }
        let sym = normalize_symbol(parts[0]);
        let z = symbol_to_z(&sym)?;
        let x: f64 = parts[1].parse().map_err(|_| format!("gjf: bad x on line {}", idx + 1))?;
        let y: f64 = parts[2].parse().map_err(|_| format!("gjf: bad y on line {}", idx + 1))?;
        let zc: f64 = parts[3].parse().map_err(|_| format!("gjf: bad z on line {}", idx + 1))?;
        atoms.push(atom_ball(z, x, y, zc));
    }
    Ok(atoms)
}

#[derive(Debug)]
struct ZmatLine {
    symbol: String,
    bond_to: Option<usize>,
    bond_len: Option<f64>,
    angle_at: Option<usize>,
    angle_deg: Option<f64>,
    dihedral_at: Option<usize>,
    dihedral_deg: Option<f64>,
}

fn parse_zmat_line(line: &str) -> Result<ZmatLine, String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return Err("gjf: empty z-matrix line".into());
    }
    let symbol = normalize_symbol(parts[0]);
    let read_usize = |s: &str| -> Result<usize, String> {
        let n: usize = s.parse().map_err(|_| format!("gjf: bad index '{s}'"))?;
        if n == 0 {
            return Err("gjf: z-matrix indices are 1-based".into());
        }
        Ok(n - 1)
    };
    let read_f64 = |s: &str| -> Result<f64, String> {
        s.parse().map_err(|_| format!("gjf: bad float '{s}'"))
    };
    Ok(ZmatLine {
        symbol,
        bond_to: parts.get(1).map(|s| read_usize(s)).transpose()?,
        bond_len: parts.get(2).map(|s| read_f64(s)).transpose()?,
        angle_at: parts.get(3).map(|s| read_usize(s)).transpose()?,
        angle_deg: parts.get(4).map(|s| read_f64(s)).transpose()?,
        dihedral_at: parts.get(5).map(|s| read_usize(s)).transpose()?,
        dihedral_deg: parts.get(6).map(|s| read_f64(s)).transpose()?,
    })
}

fn zmatrix_to_cartesian(lines: &[String]) -> Result<Vec<AtomBall>, String> {
    let entries: Vec<ZmatLine> = lines.iter().map(|l| parse_zmat_line(l)).collect::<Result<_, _>>()?;
    let n = entries.len();
    let mut pos = vec![[0.0f64; 3]; n];

    for i in 0..n {
        let e = &entries[i];
        if i == 0 {
            pos[i] = [0.0, 0.0, 0.0];
            let _ = symbol_to_z(&e.symbol)?;
            continue;
        }
        let r = e.bond_len.ok_or_else(|| format!("gjf: atom {} missing bond length", i + 1))?;
        let j = e.bond_to.ok_or_else(|| format!("gjf: atom {} missing bond atom", i + 1))?;
        if j >= i {
            return Err(format!("gjf: bond reference {} must be < atom {}", j + 1, i + 1));
        }

        if i == 1 {
            pos[i] = [0.0, 0.0, r];
            continue;
        }

        let k = e
            .angle_at
            .ok_or_else(|| format!("gjf: atom {} missing angle atom", i + 1))?;
        let theta = e
            .angle_deg
            .ok_or_else(|| format!("gjf: atom {} missing angle", i + 1))?;
        if k >= i {
            return Err(format!("gjf: angle reference {} must be < atom {}", k + 1, i + 1));
        }

        if i == 2 {
            pos[i] = place_with_angle(pos[j], pos[k], r, theta);
            continue;
        }

        let l = e
            .dihedral_at
            .ok_or_else(|| format!("gjf: atom {} missing dihedral atom", i + 1))?;
        let phi = e
            .dihedral_deg
            .ok_or_else(|| format!("gjf: atom {} missing dihedral", i + 1))?;
        if l >= k {
            return Err("gjf: dihedral atom must precede angle atom".into());
        }
        pos[i] = place_with_dihedral(pos[l], pos[k], pos[j], r, theta, phi);
    }

    Ok(entries
        .iter()
        .zip(pos.iter())
        .map(|(e, p)| {
            let z = symbol_to_z(&e.symbol).unwrap_or(0);
            atom_ball(z, p[0], p[1], p[2])
        })
        .collect())
}

fn atom_ball(element: u8, x: f64, y: f64, z: f64) -> AtomBall {
    AtomBall {
        element,
        x,
        y,
        z,
        radius: vdw_radius(element) * 0.35,
    }
}

fn place_with_angle(a: [f64; 3], b: [f64; 3], r: f64, theta_deg: f64) -> [f64; 3] {
    // Place new atom at distance r from `a`, angle theta at `a` with reference direction toward `b`.
    let theta = theta_deg.to_radians();
    let ab = sub(a, b);
    let ab_n = normalize(ab);
    let perp = normalize(cross([0.0, 1.0, 0.0], ab_n));
    let perp = if length(perp) < 1e-8 {
        normalize(cross([1.0, 0.0, 0.0], ab_n))
    } else {
        perp
    };
    let in_plane = normalize(cross(ab_n, perp));
    let dir = add(
        scale(ab_n, -theta.cos()),
        scale(in_plane, theta.sin()),
    );
    add(a, scale(normalize(dir), r))
}

fn place_with_dihedral(
    d: [f64; 3],
    c: [f64; 3],
    b: [f64; 3],
    r: f64,
    theta_deg: f64,
    phi_deg: f64,
) -> [f64; 3] {
    let theta = theta_deg.to_radians();
    let phi = phi_deg.to_radians();
    let bc = normalize(sub(b, c));
    let cd = normalize(sub(c, d));
    let n = normalize(cross(bc, cd));
    if length(n) < 1e-8 {
        return place_with_angle(b, c, r, theta_deg);
    }
    let m = cross(n, bc);
    let term1 = scale(bc, -theta.cos());
    let term2 = scale(m, theta.sin() * phi.cos());
    let term3 = scale(n, theta.sin() * phi.sin());
    let dir = normalize(add(add(term1, term2), term3));
    add(b, scale(dir, r))
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn length(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize(v: [f64; 3]) -> [f64; 3] {
    let l = length(v).max(1e-12);
    scale(v, 1.0 / l)
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Apply GaussView-style radii/bonds for rendering.
pub fn styled_geometry(mol: &MoleculeGeometry, style: MolRenderStyle) -> MoleculeGeometry {
    let mut out = mol.clone();
    match style {
        MolRenderStyle::SpaceFill => {
            for a in &mut out.atoms {
                a.radius = vdw_radius(a.element) * 0.5;
            }
            out.bonds.clear();
        }
        MolRenderStyle::Wireframe => {
            for a in &mut out.atoms {
                a.radius = 0.15;
            }
        }
        MolRenderStyle::Stick => {
            for a in &mut out.atoms {
                a.radius = 0.12;
            }
        }
        MolRenderStyle::BallAndStick => {
            for a in &mut out.atoms {
                a.radius = (vdw_radius(a.element) * 0.18).max(0.18);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER_CART: &str = r#"#p B3LYP/6-31G(d) opt

Water HF

0 1
O   0.000000   0.000000   0.000000
H   0.957200   0.000000   0.000000
H  -0.240000   0.926600   0.000000
"#;

    const WATER_ZMAT: &str = r#"# HF/6-31G(d)

water z-matrix

0 1
O
H 1 0.9572
H 1 0.9572 2 104.52
"#;

    #[test]
    fn parses_cartesian_gjf() {
        let job = parse_gjf(WATER_CART).unwrap();
        assert_eq!(job.geometry.atoms.len(), 3);
        assert_eq!(job.charge, 0);
        assert_eq!(job.multiplicity, 1);
        assert!(job.route.contains("B3LYP"));
        assert_eq!(job.coordinate_type, CoordinateType::Cartesian);
    }

    #[test]
    fn parses_zmatrix_gjf() {
        let job = parse_gjf(WATER_ZMAT).unwrap();
        assert_eq!(job.geometry.atoms.len(), 3);
        assert_eq!(job.coordinate_type, CoordinateType::ZMatrix);
        let o = &job.geometry.atoms[0];
        assert_eq!(o.element, 8);
    }
}
