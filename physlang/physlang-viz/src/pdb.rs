use crate::viewer3d::{infer_bonds, AtomBall, MoleculeGeometry};

/// Parse PDB ATOM/HETATM records (minimal subset for molecular viewer).
pub fn parse_pdb(source: &str) -> Result<MoleculeGeometry, String> {
    let mut atoms = Vec::new();
    let mut name = "pdb".to_string();

    for line in source.lines() {
        if line.starts_with("HEADER") {
            let title = line
                .strip_prefix("HEADER")
                .map(str::trim)
                .filter(|t| !t.is_empty());
            if let Some(t) = title {
                name = t.to_string();
            }
            continue;
        }
        if !(line.starts_with("ATOM") || line.starts_with("HETATM")) {
            continue;
        }
        if line.len() < 54 {
            continue;
        }
        let x: f64 = pdb_field(line, 30, 38)?.parse().map_err(|_| "pdb: bad x")?;
        let y: f64 = pdb_field(line, 38, 46)?.parse().map_err(|_| "pdb: bad y")?;
        let z: f64 = pdb_field(line, 46, 54)?.parse().map_err(|_| "pdb: bad z")?;
        let element = pdb_element(line);
        let z_num = element_symbol_to_z(&element)?;
        atoms.push(AtomBall {
            element: z_num,
            x,
            y,
            z,
            radius: 0.5,
        });
    }

    if atoms.is_empty() {
        return Err("pdb: no ATOM/HETATM records found".into());
    }

    let mut mol = MoleculeGeometry {
        name,
        atoms,
        bonds: vec![],
    };
    infer_bonds(&mut mol);
    Ok(mol)
}

pub fn parse_pdb_with_bonds(source: &str) -> Result<MoleculeGeometry, String> {
    parse_pdb(source)
}

fn pdb_field<'a>(line: &'a str, start: usize, end: usize) -> Result<&'a str, String> {
    line.get(start..end)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("pdb: missing columns {start}-{end}"))
}

fn pdb_element(line: &str) -> String {
    if line.len() >= 78 {
        let sym = line[76..78].trim();
        if !sym.is_empty() && sym.chars().all(|c| c.is_ascii_alphabetic()) {
            return normalize_symbol(sym);
        }
    }
    if line.len() >= 16 {
        let aname = line[12..16].trim();
        let letters: String = aname.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        if letters.len() >= 2 {
            let two = normalize_symbol(&letters[0..2]);
            if element_symbol_to_z(&two).is_ok() {
                return two;
            }
        }
        if let Some(c) = letters.chars().next() {
            return normalize_symbol(&c.to_string());
        }
    }
    "X".into()
}

fn normalize_symbol(sym: &str) -> String {
    let mut chars = sym.chars();
    match (chars.next(), chars.next()) {
        (Some(a), Some(b)) => format!("{}{}", a.to_ascii_uppercase(), b.to_ascii_lowercase()),
        (Some(a), None) => a.to_ascii_uppercase().to_string(),
        _ => "X".into(),
    }
}

fn element_symbol_to_z(sym: &str) -> Result<u8, String> {
    match sym {
        "H" => Ok(1),
        "He" => Ok(2),
        "C" => Ok(6),
        "N" => Ok(7),
        "O" => Ok(8),
        "F" => Ok(9),
        "Na" => Ok(11),
        "Mg" => Ok(12),
        "P" => Ok(15),
        "S" => Ok(16),
        "Cl" => Ok(17),
        "Fe" => Ok(26),
        "Cu" => Ok(29),
        "Zn" => Ok(30),
        other => Err(format!("pdb: unknown element '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WATER_PDB: &str = r"HEADER    WATER
ATOM      1  O   WAT     1       0.000   0.000   0.000  1.00  0.00           O
ATOM      2  H1  WAT     1       0.957   0.000   0.000  1.00  0.00           H
ATOM      3  H2  WAT     1      -0.240   0.927   0.000  1.00  0.00           H
END
";

    #[test]
    fn parses_water_pdb() {
        let mol = parse_pdb(WATER_PDB).unwrap();
        assert_eq!(mol.atoms.len(), 3);
        assert_eq!(mol.name, "WATER");
        assert_eq!(mol.bonds.len(), 2);
        assert_eq!(mol.atoms[0].element, 8);
    }
}
