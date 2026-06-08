use super::{parse_xyz, render_circuit_svg, CircuitSvgOptions};
use physlang_quantum::{CircuitIr, GateIr};

#[test]
fn parses_xyz_water() {
    let xyz = "3\nwater\nO 0.0 0.0 0.0\nH 0.96 0.0 0.0\nH -0.24 0.93 0.0\n";
    let mol = parse_xyz(xyz).unwrap();
    assert_eq!(mol.atoms.len(), 3);
    assert_eq!(mol.atoms[0].element, 8);
    assert_eq!(mol.bonds.len(), 2);
}

#[test]
fn parses_gjf_water() {
    let gjf = include_str!("../../../examples/molecules/water.gjf");
    let job = crate::parse_gjf(gjf).expect("water.gjf");
    assert_eq!(job.geometry.atoms.len(), 3);
    assert!(job.route.contains("B3LYP"));
}

#[test]
fn renders_simple_circuit() {
    let mut c = CircuitIr::new(2, "test");
    c.add_gate(GateIr {
        name: "H".into(),
        targets: vec![0],
        params: vec![],
    });
    c.add_gate(GateIr {
        name: "CNOT".into(),
        targets: vec![0, 1],
        params: vec![],
    });
    let svg = render_circuit_svg(&c, &CircuitSvgOptions::default());
    assert!(svg.contains("<svg"));
    assert!(svg.contains("H"));
}
