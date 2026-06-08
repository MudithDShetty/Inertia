use serde::{Deserialize, Serialize};

use crate::element::Element;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BondOrder {
    Single,
    Double,
    Triple,
    Aromatic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atom {
    pub element: u8,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub a: usize,
    pub b: usize,
    pub order: BondOrder,
}

/// Molecular graph: atoms + adjacency bonds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Molecule {
    pub name: String,
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
}

impl Molecule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            atoms: Vec::new(),
            bonds: Vec::new(),
        }
    }

    pub fn add_atom(&mut self, element: &Element, x: f64, y: f64, z: f64) -> usize {
        let idx = self.atoms.len();
        self.atoms.push(Atom {
            element: element.atomic_number,
            x,
            y,
            z,
        });
        idx
    }

    pub fn add_bond(&mut self, a: usize, b: usize, order: BondOrder) {
        self.bonds.push(Bond { a, b, order });
    }

    pub fn num_atoms(&self) -> usize {
        self.atoms.len()
    }

    pub fn bond_length(&self, bond: &Bond) -> Option<f64> {
        let a = self.atoms.get(bond.a)?;
        let b = self.atoms.get(bond.b)?;
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        Some((dx * dx + dy * dy + dz * dz).sqrt())
    }
}
