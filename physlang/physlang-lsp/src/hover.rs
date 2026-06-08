use physlang_parser::parse_source;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverInfo {
    pub line: u32,
    pub column: u32,
    pub contents: String,
}

pub fn hover_for_position(source: &str, line: u32, _column: u32) -> Option<HoverInfo> {
    let gate_docs = [
        ("H", "Hadamard gate — creates superposition"),
        ("X", "Pauli-X (NOT) gate"),
        ("Y", "Pauli-Y gate"),
        ("Z", "Pauli-Z gate"),
        ("CNOT", "Controlled-NOT two-qubit gate"),
        ("RX", "Rotation around X axis: RX(θ, qubit)"),
        ("RY", "Rotation around Y axis: RY(θ, qubit)"),
        ("RZ", "Rotation around Z axis: RZ(θ, qubit)"),
        ("expect", "Compute expectation value ⟨ψ|H|ψ⟩"),
        ("sample", "Sample measurement outcomes from circuit"),
    ];
    let line_text = source.lines().nth((line.saturating_sub(1)) as usize)?;
    for (name, doc) in gate_docs {
        if line_text.contains(name) {
            return Some(HoverInfo {
                line,
                column: 1,
                contents: doc.to_string(),
            });
        }
    }
    if parse_source(source).is_ok() {
        if line_text.contains("Energy") {
            return Some(HoverInfo {
                line,
                column: 1,
                contents: "SI type Energy — joules (kg·m²/s²)".into(),
            });
        }
    }
    None
}
