use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircuitIr {
    pub num_qubits: u32,
    pub gates: Vec<GateIr>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateIr {
    pub name: String,
    pub targets: Vec<u32>,
    pub params: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HamiltonianIr {
    pub terms: Vec<PauliTerm>,
    pub num_qubits: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PauliTerm {
    pub coeff: f64,
    pub paulis: Vec<(u32, char)>, // (qubit, 'X'|'Y'|'Z'|'I')
}

impl CircuitIr {
    pub fn new(num_qubits: u32, name: impl Into<String>) -> Self {
        Self {
            num_qubits,
            gates: Vec::new(),
            name: name.into(),
        }
    }

    pub fn add_gate(&mut self, gate: GateIr) {
        self.gates.push(gate);
    }

    pub fn to_qiskit_json(&self) -> serde_json::Value {
        serde_json::json!({
            "num_qubits": self.num_qubits,
            "name": self.name,
            "gates": self.gates,
        })
    }
}

impl HamiltonianIr {
    pub fn new(num_qubits: u32) -> Self {
        Self {
            terms: Vec::new(),
            num_qubits,
        }
    }

    pub fn add_term(&mut self, coeff: f64, paulis: Vec<(u32, char)>) {
        self.terms.push(PauliTerm { coeff, paulis });
    }
}
