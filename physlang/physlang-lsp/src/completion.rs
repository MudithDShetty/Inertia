#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Type,
    Gate,
    Function,
    Attribute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: String,
    pub kind: CompletionKind,
}

const ITEMS: &[(&str, &str, CompletionKind)] = &[
    ("fn", "define function", CompletionKind::Keyword),
    ("let", "bind variable", CompletionKind::Keyword),
    ("return", "return value", CompletionKind::Keyword),
    ("qreg", "quantum register", CompletionKind::Keyword),
    ("extern", "foreign function", CompletionKind::Keyword),
    ("if", "conditional", CompletionKind::Keyword),
    ("else", "else branch", CompletionKind::Keyword),
    ("Int", "integer type", CompletionKind::Type),
    ("Float", "floating-point type", CompletionKind::Type),
    ("Bool", "boolean type", CompletionKind::Type),
    ("Velocity", "SI velocity [L/T]", CompletionKind::Type),
    ("Force", "SI force [M·L/T²]", CompletionKind::Type),
    ("Mass", "SI mass [M]", CompletionKind::Type),
    ("Energy", "SI energy [M·L²/T²]", CompletionKind::Type),
    ("Angle", "SI angle [rad]", CompletionKind::Type),
    ("Qubit", "single qubit", CompletionKind::Type),
    ("QReg", "quantum register", CompletionKind::Type),
    ("Circuit", "quantum circuit", CompletionKind::Type),
    ("Hamiltonian", "observable Hamiltonian", CompletionKind::Type),
    ("H", "Hadamard gate", CompletionKind::Gate),
    ("X", "Pauli-X gate", CompletionKind::Gate),
    ("Y", "Pauli-Y gate", CompletionKind::Gate),
    ("Z", "Pauli-Z gate", CompletionKind::Gate),
    ("CNOT", "controlled-NOT", CompletionKind::Gate),
    ("RX", "rotation X", CompletionKind::Gate),
    ("RY", "rotation Y", CompletionKind::Gate),
    ("RZ", "rotation Z", CompletionKind::Gate),
    ("expect", "expectation value", CompletionKind::Function),
    ("sample", "measurement samples", CompletionKind::Function),
    ("ansatz", "variational ansatz", CompletionKind::Function),
    ("abs", "stdlib: absolute value", CompletionKind::Function),
    ("matmul", "stdlib: matrix multiply", CompletionKind::Function),
    ("fft", "stdlib: fast Fourier transform", CompletionKind::Function),
    ("@differentiable", "enable autodiff", CompletionKind::Attribute),
    ("@python.import", "Python FFI import", CompletionKind::Attribute),
    ("@gpu", "GPU kernel stub", CompletionKind::Attribute),
    ("@parallel", "parallel loop", CompletionKind::Attribute),
];

pub fn completions_for_prefix(source: &str, prefix: &str) -> Vec<CompletionItem> {
    let p = prefix.trim();
    let mut items: Vec<CompletionItem> = ITEMS
        .iter()
        .filter(|(label, _, _)| {
            p.is_empty() || label.to_lowercase().starts_with(&p.to_lowercase())
        })
        .map(|(label, detail, kind)| CompletionItem {
            label: (*label).to_string(),
            detail: Some((*detail).to_string()),
            insert_text: (*label).to_string(),
            kind: kind.clone(),
        })
        .collect();

    for (name, sym_kind) in crate::symbols::user_completions_for_prefix(source, prefix) {
        if items.iter().any(|i| i.label == name) {
            continue;
        }
        let kind = match sym_kind {
            crate::symbols::SymbolKind::Function | crate::symbols::SymbolKind::Extern => {
                CompletionKind::Function
            }
            crate::symbols::SymbolKind::QReg => CompletionKind::Type,
            crate::symbols::SymbolKind::Parameter | crate::symbols::SymbolKind::Variable => {
                CompletionKind::Function
            }
        };
        items.push(CompletionItem {
            label: name.clone(),
            detail: Some("user symbol".into()),
            insert_text: name,
            kind,
        });
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_by_prefix() {
        let items = completions_for_prefix("", "En");
        assert!(items.iter().any(|i| i.label == "Energy"));
        assert!(!items.iter().any(|i| i.label == "H"));
    }

    #[test]
    fn includes_user_symbols() {
        let src = "fn foo(x: Float) -> Float { let y = x; y }";
        let items = completions_for_prefix(src, "f");
        assert!(items.iter().any(|i| i.label == "foo"));
    }

    #[test]
    fn gates_available() {
        let items = completions_for_prefix("", "CN");
        assert!(items.iter().any(|i| i.label == "CNOT"));
    }
}
