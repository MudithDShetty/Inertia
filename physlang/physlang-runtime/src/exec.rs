use crate::value::RuntimeValue;
use physlang_mir::{lower_module, MirModule, MirStmt, MirValue};
use physlang_parser::parse_source;
use physlang_quantum::QuantumRuntime;
use physlang_types::check_module;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("parse error: {0}")]
    Parse(#[from] physlang_parser::ParseError),
    #[error("type error: {0}")]
    Type(#[from] physlang_types::TypeError),
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error("function '{0}' not found")]
    FunctionNotFound(String),
}

pub type ExecResult<T> = Result<T, ExecError>;

#[derive(Debug, Clone, Default)]
pub struct RuntimeOutput {
    pub stdout: Vec<String>,
    pub return_value: Option<RuntimeValue>,
    pub circuit_json: Option<String>,
}

pub fn compile_and_run(source: &str, entry: &str) -> ExecResult<RuntimeOutput> {
    let module = parse_source(source)?;
    let typed = check_module(&module)?;
    let mir = lower_module(&typed);
    execute_mir(&mir, entry)
}

pub fn compile_mir(source: &str) -> ExecResult<MirModule> {
    let module = parse_source(source)?;
    let typed = check_module(&module)?;
    Ok(lower_module(&typed))
}

fn execute_mir(mir: &MirModule, entry: &str) -> ExecResult<RuntimeOutput> {
    let func = mir
        .functions
        .iter()
        .find(|f| f.name == entry)
        .ok_or_else(|| ExecError::FunctionNotFound(entry.to_string()))?;

    let mut env: HashMap<String, RuntimeValue> = HashMap::new();
    let mut quantum = QuantumRuntime::new();
    for (name, size) in &mir.qregs {
        quantum.alloc_register(name, *size);
    }
    for (name, val) in &mir.globals {
        env.insert(name.clone(), eval_mir_value(val, &env, &mut quantum)?);
    }

    let mut output = RuntimeOutput::default();
    for stmt in &func.body {
        match stmt {
            MirStmt::Let { name, value } => {
                env.insert(name.clone(), eval_mir_value(value, &env, &mut quantum)?);
            }
            MirStmt::Return { value } => {
                output.return_value = value
                    .as_ref()
                    .map(|v| eval_mir_value(v, &env, &mut quantum))
                    .transpose()?;
            }
            MirStmt::Expr(v) => {
                let rv = eval_mir_value(v, &env, &mut quantum)?;
                output.stdout.push(rv.display());
            }
        }
    }

    if let Some(circ) = quantum.last_circuit_json() {
        output.circuit_json = Some(circ);
    }
    Ok(output)
}

fn eval_mir_value(
    val: &MirValue,
    env: &HashMap<String, RuntimeValue>,
    quantum: &mut QuantumRuntime,
) -> ExecResult<RuntimeValue> {
    use physlang_mir::{MirBinaryOp, MirUnaryOp};
    match val {
        MirValue::Int(v) => Ok(RuntimeValue::Int(*v)),
        MirValue::Float(v) => Ok(RuntimeValue::Float(*v)),
        MirValue::Bool(v) => Ok(RuntimeValue::Bool(*v)),
        MirValue::String(s) => Ok(RuntimeValue::String(s.clone())),
        MirValue::Quantity { value, unit } => Ok(RuntimeValue::Quantity {
            value: *value,
            unit: unit.clone(),
        }),
        MirValue::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| ExecError::Runtime(format!("undefined variable '{name}'"))),
        MirValue::Unary { op, expr } => {
            let v = eval_mir_value(expr, env, quantum)?;
            match op {
                MirUnaryOp::Neg => Ok(RuntimeValue::Float(-v.as_f64().unwrap_or(0.0))),
                MirUnaryOp::Not => Ok(RuntimeValue::Bool(!matches!(v, RuntimeValue::Bool(true)))),
            }
        }
        MirValue::Binary { op, left, right } => {
            let l = eval_mir_value(left, env, quantum)?;
            let r = eval_mir_value(right, env, quantum)?;
            match op {
                MirBinaryOp::Add => Ok(RuntimeValue::Float(l.as_f64().unwrap_or(0.0) + r.as_f64().unwrap_or(0.0))),
                MirBinaryOp::Sub => Ok(RuntimeValue::Float(l.as_f64().unwrap_or(0.0) - r.as_f64().unwrap_or(0.0))),
                MirBinaryOp::Mul => Ok(RuntimeValue::Float(l.as_f64().unwrap_or(0.0) * r.as_f64().unwrap_or(0.0))),
                MirBinaryOp::Div => Ok(RuntimeValue::Float(l.as_f64().unwrap_or(0.0) / r.as_f64().unwrap_or(0.0))),
                MirBinaryOp::At => {
                    quantum.tensor_hamiltonians();
                    Ok(RuntimeValue::Quantum(physlang_quantum::QuantumValue::Hamiltonian(
                        "tensor".into(),
                    )))
                }
            }
        }
        MirValue::Gate { name, targets, params } => {
            let p: Vec<f64> = params
                .iter()
                .map(|pv| {
                    eval_mir_value(pv, env, quantum).and_then(|v| {
                        v.as_f64()
                            .ok_or_else(|| ExecError::Runtime("gate param must be numeric".into()))
                    })
                })
                .collect::<ExecResult<_>>()?;
            quantum.apply_gate(name, targets, &p);
            Ok(RuntimeValue::Quantum(physlang_quantum::QuantumValue::Gate(
                name.clone(),
            )))
        }
        MirValue::Call { name, args } => {
            let evaluated: Vec<RuntimeValue> = args
                .iter()
                .map(|a| eval_mir_value(a, env, quantum))
                .collect::<ExecResult<_>>()?;
            match name.as_str() {
                "expect" => {
                    let params: Vec<f64> = evaluated.iter().filter_map(|v| v.as_f64()).collect();
                    let energy = quantum
                        .expectation(&params)
                        .map_err(|e| ExecError::Runtime(e))?;
                    Ok(RuntimeValue::Float(energy))
                }
                "sample" => {
                    let shots = evaluated.get(1).and_then(|v| v.as_f64()).unwrap_or(1024.0) as u32;
                    let counts = quantum
                        .sample(shots)
                        .map_err(|e| ExecError::Runtime(e))?;
                    Ok(RuntimeValue::String(counts))
                }
                "ansatz" => {
                    let params: Vec<f64> = evaluated.iter().filter_map(|v| v.as_f64()).collect();
                    let circ = quantum
                        .build_ansatz(&params)
                        .map_err(|e| ExecError::Runtime(e))?;
                    Ok(RuntimeValue::Quantum(physlang_quantum::QuantumValue::Circuit(circ)))
                }
                "print" => {
                    let s = evaluated.first().map(|v| v.display()).unwrap_or_default();
                    Ok(RuntimeValue::String(s))
                }
                _ => Err(ExecError::Runtime(format!("unknown function '{name}'"))),
            }
        }
        MirValue::Tensor { left, right } => {
            eval_mir_value(left, env, quantum)?;
            eval_mir_value(right, env, quantum)?;
            Ok(RuntimeValue::Quantum(physlang_quantum::QuantumValue::Hamiltonian(
                "term".into(),
            )))
        }
    }
}
