//! Periodic table helpers for molecular file parsers.

pub fn normalize_symbol(sym: &str) -> String {
    let sym = sym.trim();
    let mut chars = sym.chars().filter(|c| c.is_ascii_alphabetic());
    match (chars.next(), chars.next()) {
        (Some(a), Some(b)) => format!("{}{}", a.to_ascii_uppercase(), b.to_ascii_lowercase()),
        (Some(a), None) => a.to_ascii_uppercase().to_string(),
        _ => "X".into(),
    }
}

pub fn symbol_to_z(sym: &str) -> Result<u8, String> {
    let s = normalize_symbol(sym);
    match s.as_str() {
        "H" => Ok(1),
        "He" => Ok(2),
        "Li" => Ok(3),
        "Be" => Ok(4),
        "B" => Ok(5),
        "C" => Ok(6),
        "N" => Ok(7),
        "O" => Ok(8),
        "F" => Ok(9),
        "Ne" => Ok(10),
        "Na" => Ok(11),
        "Mg" => Ok(12),
        "Al" => Ok(13),
        "Si" => Ok(14),
        "P" => Ok(15),
        "S" => Ok(16),
        "Cl" => Ok(17),
        "Ar" => Ok(18),
        "K" => Ok(19),
        "Ca" => Ok(20),
        "Fe" => Ok(26),
        "Cu" => Ok(29),
        "Zn" => Ok(30),
        "Br" => Ok(35),
        "I" => Ok(53),
        other if other.len() <= 2 => Err(format!("unknown element '{other}'")),
        _ => Err(format!("unknown element '{sym}'")),
    }
}

pub fn z_to_symbol(z: u8) -> &'static str {
    match z {
        1 => "H",
        2 => "He",
        3 => "Li",
        6 => "C",
        7 => "N",
        8 => "O",
        9 => "F",
        11 => "Na",
        12 => "Mg",
        15 => "P",
        16 => "S",
        17 => "Cl",
        26 => "Fe",
        29 => "Cu",
        30 => "Zn",
        35 => "Br",
        53 => "I",
        _ => "X",
    }
}

/// Covalent radii (Å) for bond inference.
pub fn covalent_radius(z: u8) -> f64 {
    match z {
        1 => 0.31,
        6 => 0.76,
        7 => 0.71,
        8 => 0.66,
        9 => 0.57,
        16 => 1.05,
        17 => 1.02,
        _ => 0.75,
    }
}

/// Van der Waals radii (Å) for space-filling display (GaussView CPK-style).
pub fn vdw_radius(z: u8) -> f64 {
    match z {
        1 => 1.20,
        6 => 1.70,
        7 => 1.55,
        8 => 1.52,
        9 => 1.47,
        16 => 1.80,
        17 => 1.75,
        _ => 1.60,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_roundtrip_common() {
        assert_eq!(symbol_to_z("O").unwrap(), 8);
        assert_eq!(symbol_to_z("cl").unwrap(), 17);
        assert_eq!(z_to_symbol(8), "O");
    }
}
