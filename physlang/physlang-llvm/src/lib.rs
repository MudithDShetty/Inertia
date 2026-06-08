//! PhysicsLang LLVM IR codegen (optional feature).

use physlang_mir::{MirFunction, MirModule, MirValue};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlvmError {
    #[error("LLVM backend not enabled — rebuild with --features llvm")]
    NotEnabled,
    #[error("codegen error: {0}")]
    Codegen(String),
}

pub fn emit_llvm_ir(mir: &MirModule) -> Result<String, LlvmError> {
    #[cfg(feature = "llvm")]
    {
        return emit_llvm_ir_impl(mir);
    }
    #[cfg(not(feature = "llvm"))]
    {
        let _ = mir;
        Ok(generate_pseudo_llvm(mir))
    }
}

#[cfg(feature = "llvm")]
fn emit_llvm_ir_impl(_mir: &MirModule) -> Result<String, LlvmError> {
    Err(LlvmError::Codegen(
        "inkwell integration stub — use pseudo-IR output".into(),
    ))
}

/// Portable pseudo-LLVM IR for debugging when LLVM is not linked.
pub fn generate_pseudo_llvm(mir: &MirModule) -> String {
    let mut out = String::from("; PhysicsLang pseudo-LLVM IR\n");
    for func in &mir.functions {
        out.push_str(&emit_function(func));
    }
    out
}

fn emit_function(func: &MirFunction) -> String {
    let params: Vec<String> = func.params.iter().map(|p| format!("i64 %{p}")).collect();
    let mut body = String::new();
    for stmt in &func.body {
        use physlang_mir::MirStmt;
        match stmt {
            MirStmt::Let { name, value } => {
                body.push_str(&format!("  ; let {name} = {}\n", emit_value(value)));
            }
            MirStmt::Return { value } => {
                if let Some(v) = value {
                    body.push_str(&format!("  ret i64 {}\n", emit_value(v)));
                } else {
                    body.push_str("  ret i64 0\n");
                }
            }
            MirStmt::Expr(v) => {
                body.push_str(&format!("  ; expr {}\n", emit_value(v)));
            }
        }
    }
    format!(
        "define i64 @{}({}) {{\n{body}}}\n",
        func.name,
        params.join(", ")
    )
}

fn emit_value(val: &MirValue) -> String {
    match val {
        MirValue::Int(v) => v.to_string(),
        MirValue::Float(v) => format!("{v}"),
        MirValue::Ident(s) => format!("%{s}"),
        MirValue::Call { name, .. } => format!("call @{name}()"),
        _ => "0".into(),
    }
}
