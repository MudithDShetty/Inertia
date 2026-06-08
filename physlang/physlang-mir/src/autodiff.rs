use crate::lower::{MirFunction, MirModule, MirValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffMode {
    ParameterShift,
    Adjoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffProgram {
    pub function: String,
    pub mode: DiffMode,
    pub param_names: Vec<String>,
    pub gradient_stmts: Vec<GradientStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GradientStep {
    ParameterShift {
        gate: String,
        target: u32,
        shift: f64,
    },
    AdjointExpect {
        hamiltonian: String,
        circuit: String,
    },
    AccumulateGradient {
        param: String,
        contribution: MirValue,
    },
}

pub fn autodiff_function(mir: &MirModule, name: &str) -> Option<DiffProgram> {
    let func = mir.functions.iter().find(|f| f.name == name)?;
    if !func.is_differentiable {
        return None;
    }

    let param_names: Vec<String> = func
        .params
        .iter()
        .filter(|p| p.starts_with("theta") || p.contains("param") || p.as_str() == "phi")
        .cloned()
        .collect();

    let params = if param_names.is_empty() {
        func.params.clone()
    } else {
        param_names
    };

    let mut gradient_stmts = Vec::new();
    for stmt in &func.body {
        collect_diff_steps(stmt, &mut gradient_stmts);
    }

    for p in &params {
        gradient_stmts.push(GradientStep::AccumulateGradient {
            param: p.clone(),
            contribution: MirValue::Float(0.0),
        });
    }

    Some(DiffProgram {
        function: name.to_string(),
        mode: DiffMode::ParameterShift,
        param_names: params,
        gradient_stmts,
    })
}

fn collect_diff_steps(stmt: &crate::lower::MirStmt, out: &mut Vec<GradientStep>) {
    use crate::lower::MirStmt;
    match stmt {
        MirStmt::Return {
            value: Some(MirValue::Call { name, args, .. }),
        } if name == "expect" => {
            out.push(GradientStep::AdjointExpect {
                hamiltonian: format!("{:?}", args.get(1)),
                circuit: format!("{:?}", args.first()),
            });
        }
        MirStmt::Let { value, .. } | MirStmt::Expr(value) | MirStmt::Return { value: Some(value) } => {
            collect_from_value(value, out);
        }
        _ => {}
    }
}

fn collect_from_value(value: &MirValue, out: &mut Vec<GradientStep>) {
    match value {
        MirValue::Gate { name, targets, params } if is_parametric_gate(name) => {
            for t in targets {
                out.push(GradientStep::ParameterShift {
                    gate: name.clone(),
                    target: *t,
                    shift: std::f64::consts::PI / 2.0,
                });
            }
            for p in params {
                collect_from_value(p, out);
            }
        }
        MirValue::Call { args, .. } => {
            for a in args {
                collect_from_value(a, out);
            }
        }
        MirValue::Binary { left, right, .. } => {
            collect_from_value(left, out);
            collect_from_value(right, out);
        }
        MirValue::Unary { expr, .. } => collect_from_value(expr, out),
        MirValue::Tensor { left, right } => {
            collect_from_value(left, out);
            collect_from_value(right, out);
        }
        _ => {}
    }
}

fn is_parametric_gate(name: &str) -> bool {
    matches!(name, "RX" | "RY" | "RZ" | "U3")
}

#[cfg(test)]
mod tests {
    use super::*;
    use physlang_mir::lower_module;
    use physlang_parser::parse_source;
    use physlang_types::check_module;

    #[test]
    fn autodiff_energy_fn() {
        let src = r#"
qreg q[2]
let H = X(0) @ X(1)

@differentiable
fn energy(theta: Angle[3]) -> Energy {
    let circuit = ansatz(q, theta)
    return expect(circuit, H)
}
"#;
        let module = parse_source(src).unwrap();
        let typed = check_module(&module).unwrap();
        let mir = lower_module(&typed);
        let diff = autodiff_function(&mir, "energy").unwrap();
        assert_eq!(diff.function, "energy");
        assert!(!diff.param_names.is_empty());
    }
}
