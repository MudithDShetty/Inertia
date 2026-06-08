use serde::{Deserialize, Serialize};

/// Chemical element from the periodic table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub atomic_number: u8,
    pub symbol: &'static str,
    pub name: &'static str,
    /// Standard atomic weight (u).
    pub atomic_mass: f64,
    /// Covalent radius (pm); 0 if unknown in stub data.
    pub covalent_radius_pm: f64,
    /// Pauling electronegativity; NaN if unknown.
    pub electronegativity: f64,
}

/// First-row and common organic/main-group elements (stub subset).
const TABLE: &[Element] = &[
    elem(1, "H", "Hydrogen", 1.008, 31.0, 2.20),
    elem(2, "He", "Helium", 4.003, 28.0, f64::NAN),
    elem(6, "C", "Carbon", 12.011, 76.0, 2.55),
    elem(7, "N", "Nitrogen", 14.007, 71.0, 3.04),
    elem(8, "O", "Oxygen", 15.999, 66.0, 3.44),
    elem(9, "F", "Fluorine", 18.998, 57.0, 3.98),
    elem(10, "Ne", "Neon", 20.180, 58.0, f64::NAN),
    elem(11, "Na", "Sodium", 22.990, 166.0, 0.93),
    elem(12, "Mg", "Magnesium", 24.305, 141.0, 1.31),
    elem(15, "P", "Phosphorus", 30.974, 107.0, 2.19),
    elem(16, "S", "Sulfur", 32.06, 105.0, 2.58),
    elem(17, "Cl", "Chlorine", 35.45, 102.0, 3.16),
    elem(18, "Ar", "Argon", 39.948, 106.0, f64::NAN),
    elem(26, "Fe", "Iron", 55.845, 132.0, 1.83),
    elem(29, "Cu", "Copper", 63.546, 132.0, 1.90),
    elem(30, "Zn", "Zinc", 65.38, 122.0, 1.65),
];

const fn elem(
    atomic_number: u8,
    symbol: &'static str,
    name: &'static str,
    atomic_mass: f64,
    covalent_radius_pm: f64,
    electronegativity: f64,
) -> Element {
    Element {
        atomic_number,
        symbol,
        name,
        atomic_mass,
        covalent_radius_pm,
        electronegativity,
    }
}

pub fn periodic_table() -> &'static [Element] {
    TABLE
}

pub fn element_by_symbol(symbol: &str) -> Option<&'static Element> {
    TABLE.iter().find(|e| e.symbol.eq_ignore_ascii_case(symbol))
}

pub fn element_by_number(z: u8) -> Option<&'static Element> {
    TABLE.iter().find(|e| e.atomic_number == z)
}
