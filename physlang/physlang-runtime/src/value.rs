use physlang_mir::MirValue;
use physlang_quantum::QuantumValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Quantity { value: f64, unit: String },
    Quantum(QuantumValue),
    Unit,
}

impl RuntimeValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            RuntimeValue::Float(v) | RuntimeValue::Quantity { value: v, .. } => Some(*v),
            RuntimeValue::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn display(&self) -> String {
        match self {
            RuntimeValue::Int(v) => v.to_string(),
            RuntimeValue::Float(v) => v.to_string(),
            RuntimeValue::Bool(v) => v.to_string(),
            RuntimeValue::String(s) => s.clone(),
            RuntimeValue::Quantity { value, unit } => format!("{value} {unit}"),
            RuntimeValue::Quantum(q) => q.display(),
            RuntimeValue::Unit => "()".into(),
        }
    }
}

impl From<MirValue> for RuntimeValue {
    fn from(m: MirValue) -> Self {
        match m {
            MirValue::Int(v) => RuntimeValue::Int(v),
            MirValue::Float(v) => RuntimeValue::Float(v),
            MirValue::Bool(v) => RuntimeValue::Bool(v),
            MirValue::String(s) => RuntimeValue::String(s),
            MirValue::Quantity { value, unit } => RuntimeValue::Quantity { value, unit },
            _ => RuntimeValue::Unit,
        }
    }
}
