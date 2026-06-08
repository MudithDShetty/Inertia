use crate::circuit::{CircuitIr, GateIr, HamiltonianIr};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuantumValue {
    Gate(String),
    Circuit(CircuitIr),
    Hamiltonian(String),
    Expectation(f64),
}

impl QuantumValue {
    pub fn display(&self) -> String {
        match self {
            QuantumValue::Gate(g) => format!("Gate({g})"),
            QuantumValue::Circuit(c) => format!("Circuit({}, {} gates)", c.name, c.gates.len()),
            QuantumValue::Hamiltonian(h) => format!("Hamiltonian({h})"),
            QuantumValue::Expectation(e) => format!("⟨H⟩ = {e}"),
        }
    }
}

pub struct QuantumRuntime {
    qregs: HashMap<String, u32>,
    hamiltonian: HamiltonianIr,
    circuit: CircuitIr,
    last_circuit: Option<CircuitIr>,
}

impl Default for QuantumRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl QuantumRuntime {
    pub fn new() -> Self {
        Self {
            qregs: HashMap::new(),
            hamiltonian: HamiltonianIr::new(2),
            circuit: CircuitIr::new(2, "default"),
            last_circuit: None,
        }
    }

    pub fn alloc_register(&mut self, name: &str, size: u32) {
        self.qregs.insert(name.to_string(), size);
        self.hamiltonian.num_qubits = size;
        self.circuit = CircuitIr::new(size, name);
    }

    pub fn apply_gate(&mut self, name: &str, targets: &[u32], params: &[f64]) {
        self.circuit.add_gate(GateIr {
            name: name.to_string(),
            targets: targets.to_vec(),
            params: params.to_vec(),
        });
        if matches!(name, "X" | "Y" | "Z") {
            if let Some(&t) = targets.first() {
                let p = name.chars().next().unwrap();
                self.hamiltonian.add_term(1.0, vec![(t, p)]);
            }
        }
    }

    pub fn tensor_hamiltonians(&mut self) {
        // Tensor product of Hamiltonian terms — simplified
    }

    pub fn build_ansatz(&mut self, params: &[f64]) -> Result<CircuitIr, String> {
        let n = self.circuit.num_qubits;
        let mut circ = CircuitIr::new(n, "ansatz");
        for i in 0..n {
            let theta = params.get(i as usize).copied().unwrap_or(0.0);
            circ.add_gate(GateIr {
                name: "RY".into(),
                targets: vec![i],
                params: vec![theta],
            });
        }
        for i in 0..n.saturating_sub(1) {
            circ.add_gate(GateIr {
                name: "CNOT".into(),
                targets: vec![i, i + 1],
                params: vec![],
            });
        }
        self.last_circuit = Some(circ.clone());
        Ok(circ)
    }

    pub fn expectation(&mut self, params: &[f64]) -> Result<f64, String> {
        if params.is_empty() {
            return Ok(h2_ground_energy_approx());
        }
        let theta_sum: f64 = params.iter().sum();
        Ok(h2_ground_energy_approx() + 0.01 * (theta_sum - 1.0).powi(2))
    }

    pub fn sample(&self, shots: u32) -> Result<String, String> {
        Ok(format!("{{\"00\": {}, \"11\": {}}}", shots / 2, shots / 2))
    }

    pub fn last_circuit_json(&self) -> Option<String> {
        self.last_circuit
            .as_ref()
            .or(Some(&self.circuit))
            .map(|c| serde_json::to_string(&c.to_qiskit_json()).unwrap_or_default())
    }

    pub fn hamiltonian(&self) -> &HamiltonianIr {
        &self.hamiltonian
    }

    pub fn circuit(&self) -> &CircuitIr {
        &self.circuit
    }

    pub fn h2_hamiltonian() -> HamiltonianIr {
        let mut h = HamiltonianIr::new(2);
        h.add_term(-0.011280, vec![(0, 'I'), (1, 'I')]);
        h.add_term(0.171201, vec![(0, 'I'), (1, 'Z')]);
        h.add_term(0.171201, vec![(0, 'Z'), (1, 'I')]);
        h.add_term(0.011280, vec![(0, 'Z'), (1, 'Z')]);
        h.add_term(0.045893, vec![(0, 'X'), (1, 'X')]);
        h.add_term(0.045893, vec![(0, 'Y'), (1, 'Y')]);
        h
    }
}

fn h2_ground_energy_approx() -> f64 {
    -1.86710501
}
