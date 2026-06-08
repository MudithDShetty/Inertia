//! PhysicsAtom — periodic table, molecular graphs, force fields (Phase 3 stub).

mod element;
mod molecule;

pub use element::{element_by_number, element_by_symbol, periodic_table, Element};
pub use molecule::{Atom, Bond, BondOrder, Molecule};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_carbon() {
        let c = element_by_symbol("C").expect("carbon");
        assert_eq!(c.atomic_number, 6);
        assert!((c.atomic_mass - 12.011).abs() < 0.01);
    }

    #[test]
    fn water_stub_geometry() {
        let o = element_by_symbol("O").unwrap();
        let h = element_by_symbol("H").unwrap();
        let mut water = Molecule::new("water");
        let oi = water.add_atom(o, 0.0, 0.0, 0.0);
        let h1 = water.add_atom(h, 0.96, 0.0, 0.0);
        let h2 = water.add_atom(h, -0.24, 0.93, 0.0);
        water.add_bond(oi, h1, BondOrder::Single);
        water.add_bond(oi, h2, BondOrder::Single);
        assert_eq!(water.num_atoms(), 3);
        assert!((water.bond_length(&water.bonds[0]).unwrap() - 0.96).abs() < 1e-6);
    }
}
