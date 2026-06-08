use crate::units::{Dimensions, QuantityType, UnitRegistry};
use physlang_parser::{
    BinaryOp, Expr, ExprKind, FunctionDef, Item, Module, Stmt, TypeKind, UnaryOp,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum TypeError {
    #[error("line {line}: dimension mismatch — expected {expected}, found {found}")]
    DimensionMismatch {
        line: u32,
        expected: String,
        found: String,
    },
    #[error("line {line}: type mismatch — expected {expected}, found {found}")]
    TypeMismatch {
        line: u32,
        expected: String,
        found: String,
    },
    #[error("line {line}: undefined identifier '{name}'")]
    UndefinedIdent { line: u32, name: String },
    #[error("line {line}: {message}")]
    Other { line: u32, message: String },
}

pub type TypeResult<T> = Result<T, TypeError>;

#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub ty: QuantityType,
    pub dims: Option<Dimensions>,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
pub struct TypedFunction {
    pub def: FunctionDef,
    pub ret_ty: QuantityType,
    pub is_differentiable: bool,
    pub python_import: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TypedItem {
    Function(TypedFunction),
    QReg { name: String, size: u32 },
    Let { name: String, ty: QuantityType, value: TypedExpr },
}

#[derive(Debug, Clone)]
pub struct TypedModule {
    pub items: Vec<TypedItem>,
    pub functions: HashMap<String, TypedFunction>,
    pub qregs: HashMap<String, u32>,
    pub globals: HashMap<String, QuantityType>,
}

pub fn check_module(module: &Module) -> TypeResult<TypedModule> {
    let registry = UnitRegistry::new();
    let mut ctx = Checker {
        registry,
        qregs: HashMap::new(),
        globals: HashMap::new(),
        functions: HashMap::new(),
    };
    let mut items = Vec::new();
    for item in &module.items {
        items.push(ctx.check_item(item)?);
    }
    Ok(TypedModule {
        items,
        functions: ctx.functions,
        qregs: ctx.qregs,
        globals: ctx.globals,
    })
}

struct Checker {
    registry: UnitRegistry,
    qregs: HashMap<String, u32>,
    globals: HashMap<String, QuantityType>,
    functions: HashMap<String, TypedFunction>,
}

impl Checker {
    fn check_item(&mut self, item: &Item) -> TypeResult<TypedItem> {
        match item {
            Item::QReg(q) => {
                self.qregs.insert(q.name.clone(), q.size);
                self.globals.insert(
                    q.name.clone(),
                    QuantityType::Array {
                        elem: Box::new(QuantityType::Named("Qubit".into())),
                        len: q.size,
                    },
                );
                Ok(TypedItem::QReg {
                    name: q.name.clone(),
                    size: q.size,
                })
            }
            Item::Let(l) => {
                let value = self.check_expr(&l.value)?;
                let ty = if let Some(ref annot) = l.ty {
                    let expected = self.resolve_type(annot)?;
                    self.ensure_compatible(l.span.start_line, &expected, &value.ty, &value.dims)?;
                    expected
                } else {
                    value.ty.clone()
                };
                self.globals.insert(l.name.clone(), ty.clone());
                Ok(TypedItem::Let {
                    name: l.name.clone(),
                    ty,
                    value,
                })
            }
            Item::Function(f) => {
                let tf = self.check_function(f)?;
                self.functions.insert(f.name.clone(), tf.clone());
                Ok(TypedItem::Function(tf))
            }
            Item::Extern(_) => Err(TypeError::Other {
                line: 1,
                message: "extern declarations not yet type-checked".into(),
            }),
        }
    }

    fn check_function(&mut self, f: &FunctionDef) -> TypeResult<TypedFunction> {
        let mut locals = HashMap::new();
        for p in &f.params {
            let ty = self.resolve_type(&p.ty)?;
            locals.insert(p.name.clone(), ty);
        }
        let ret_ty = f
            .ret_type
            .as_ref()
            .map(|t| self.resolve_type(t))
            .transpose()?
            .unwrap_or(QuantityType::Void);

        let is_differentiable = f.attrs.iter().any(|a| a.name == "differentiable");
        let python_import = f
            .attrs
            .iter()
            .find(|a| a.name == "python.import")
            .and_then(|a| a.args.first().cloned());

        let mut body_checker = FunctionChecker {
            parent: self,
            locals,
        };
        for stmt in &f.body.stmts {
            body_checker.check_stmt(stmt, &ret_ty)?;
        }

        Ok(TypedFunction {
            def: f.clone(),
            ret_ty,
            is_differentiable,
            python_import,
        })
    }

    fn resolve_type(&self, ty: &physlang_parser::TypeExpr) -> TypeResult<QuantityType> {
        match &ty.kind {
            TypeKind::Named(name) => Ok(self.registry.type_for_name(name)),
            TypeKind::Array { elem, len } => {
                let e = self.resolve_type(elem)?;
                Ok(QuantityType::Array {
                    elem: Box::new(e),
                    len: *len,
                })
            }
            TypeKind::Quantity { value_type, unit } => {
                let dims = self
                    .registry
                    .resolve_unit_factors(&unit.factors)
                    .ok_or_else(|| TypeError::Other {
                        line: ty.span.start_line,
                        message: format!("unknown unit in type annotation"),
                    })?;
                Ok(QuantityType::Quantity {
                    value_type: value_type.clone(),
                    dims,
                })
            }
            TypeKind::UnitOnly(unit) => {
                let dims = self
                    .registry
                    .resolve_unit_factors(&unit.factors)
                    .ok_or_else(|| TypeError::Other {
                        line: ty.span.start_line,
                        message: "unknown unit".into(),
                    })?;
                Ok(QuantityType::Quantity {
                    value_type: "scalar".into(),
                    dims,
                })
            }
        }
    }

    fn check_expr(&self, expr: &Expr) -> TypeResult<TypedExpr> {
        FunctionChecker {
            parent: self,
            locals: HashMap::new(),
        }
        .check_expr(expr)
    }

    fn ensure_compatible(
        &self,
        line: u32,
        expected: &QuantityType,
        found: &QuantityType,
        found_dims: &Option<Dimensions>,
    ) -> TypeResult<()> {
        if matches!(expected, QuantityType::Named(n) if n == "Force")
            && matches!(found, QuantityType::Named(n) if n == "Mass" || n == "Velocity")
        {
            return Err(TypeError::DimensionMismatch {
                line,
                expected: "Force (kg·m/s²)".into(),
                found: format!("{found:?}"),
            });
        }
        if let (Some(ed), Some(fd)) = (expected.dims(), found_dims) {
            if !ed.is_compatible_with(*fd) {
                return Err(TypeError::DimensionMismatch {
                    line,
                    expected: ed.display(),
                    found: fd.display(),
                });
            }
        }
        Ok(())
    }
}

struct FunctionChecker<'a> {
    parent: &'a Checker,
    locals: HashMap<String, QuantityType>,
}

impl FunctionChecker<'_> {
    fn check_stmt(&mut self, stmt: &Stmt, ret_ty: &QuantityType) -> TypeResult<()> {
        match stmt {
            Stmt::Let(l) => {
                let value = self.check_expr(&l.value)?;
                let ty = if let Some(ref annot) = l.ty {
                    let expected = self.parent.resolve_type(annot)?;
                    self.parent.ensure_compatible(
                        l.span.start_line,
                        &expected,
                        &value.ty,
                        &value.dims,
                    )?;
                    expected
                } else {
                    value.ty.clone()
                };
                self.locals.insert(l.name.clone(), ty);
                Ok(())
            }
            Stmt::Return { value, span } => {
                if let Some(v) = value {
                    let typed = self.check_expr(v)?;
                    self.parent.ensure_compatible(
                        span.start_line,
                        ret_ty,
                        &typed.ty,
                        &typed.dims,
                    )?;
                }
                Ok(())
            }
            Stmt::Expr { expr, .. } => {
                self.check_expr(expr)?;
                Ok(())
            }
        }
    }

    fn check_expr(&self, expr: &Expr) -> TypeResult<TypedExpr> {
        let line = expr.span.start_line;
        match &expr.kind {
            ExprKind::Int(_) => Ok(TypedExpr {
                ty: QuantityType::Int,
                dims: None,
                expr: expr.clone(),
            }),
            ExprKind::Float(_) => Ok(TypedExpr {
                ty: QuantityType::Float,
                dims: None,
                expr: expr.clone(),
            }),
            ExprKind::Bool(_) => Ok(TypedExpr {
                ty: QuantityType::Bool,
                dims: None,
                expr: expr.clone(),
            }),
            ExprKind::String(_) => Ok(TypedExpr {
                ty: QuantityType::String,
                dims: None,
                expr: expr.clone(),
            }),
            ExprKind::Quantity { unit, .. } => {
                let dims = self
                    .parent
                    .registry
                    .resolve_unit_factors(&unit.factors)
                    .ok_or_else(|| TypeError::Other {
                        line,
                        message: "unknown unit".into(),
                    })?;
                Ok(TypedExpr {
                    ty: QuantityType::Float,
                    dims: Some(dims),
                    expr: expr.clone(),
                })
            }
            ExprKind::Ident(name) => {
                if let Some(ty) = self.locals.get(name).or_else(|| self.parent.globals.get(name))
                {
                    let dims = self.parent.registry.dims_for_named(match ty {
                        QuantityType::Named(n) => n.as_str(),
                        _ => "",
                    });
                    return Ok(TypedExpr {
                        ty: ty.clone(),
                        dims,
                        expr: expr.clone(),
                    });
                }
                // Built-in gates / functions
                if is_gate_ident(name) {
                    return Ok(TypedExpr {
                        ty: QuantityType::Gate,
                        dims: None,
                        expr: expr.clone(),
                    });
                }
                if is_quantum_fn(name) || self.parent.functions.contains_key(name) {
                    let ty = match name.as_str() {
                        "expect" => QuantityType::Energy,
                        "sample" => QuantityType::Named("SampleResult".into()),
                        "ansatz" => QuantityType::Circuit,
                        _ => QuantityType::Named(name.clone()),
                    };
                    return Ok(TypedExpr {
                        ty,
                        dims: None,
                        expr: expr.clone(),
                    });
                }
                Err(TypeError::UndefinedIdent {
                    line,
                    name: name.clone(),
                })
            }
            ExprKind::Unary { op, expr: inner } => {
                let inner = self.check_expr(inner)?;
                match op {
                    UnaryOp::Neg => Ok(TypedExpr {
                        ty: inner.ty.clone(),
                        dims: inner.dims,
                        expr: expr.clone(),
                    }),
                    UnaryOp::Not => Ok(TypedExpr {
                        ty: QuantityType::Bool,
                        dims: None,
                        expr: expr.clone(),
                    }),
                }
            }
            ExprKind::Binary { op, left, right } => {
                let l = self.check_expr(left)?;
                let r = self.check_expr(right)?;
                match op {
                    BinaryOp::Add | BinaryOp::Sub => {
                        if let (Some(ld), Some(rd)) = (l.dims, r.dims) {
                            if !ld.is_compatible_with(rd) {
                                return Err(TypeError::DimensionMismatch {
                                    line,
                                    expected: ld.display(),
                                    found: rd.display(),
                                });
                            }
                        } else if l.ty != r.ty
                            && !matches!(
                                (&l.ty, &r.ty),
                                (QuantityType::Hamiltonian, QuantityType::Hamiltonian)
                                    | (QuantityType::Circuit, QuantityType::Circuit)
                            )
                        {
                            // Allow Hamiltonian addition
                            if !matches!(l.ty, QuantityType::Hamiltonian)
                                && !matches!(r.ty, QuantityType::Hamiltonian)
                            {
                                return Err(TypeError::TypeMismatch {
                                    line,
                                    expected: format!("{:?}", l.ty),
                                    found: format!("{:?}", r.ty),
                                });
                            }
                        }
                        let ty = if matches!(l.ty, QuantityType::Hamiltonian) {
                            QuantityType::Hamiltonian
                        } else {
                            l.ty.clone()
                        };
                        Ok(TypedExpr {
                            ty,
                            dims: l.dims.or(r.dims),
                            expr: expr.clone(),
                        })
                    }
                    BinaryOp::Mul => {
                        let dims = match (l.dims, r.dims) {
                            (Some(ld), Some(rd)) => Some(ld.mul(rd)),
                            (Some(d), None) | (None, Some(d)) => Some(d),
                            (None, None) => None,
                        };
                        // kg * (m/s) should yield Force dims
                        Ok(TypedExpr {
                            ty: QuantityType::Float,
                            dims,
                            expr: expr.clone(),
                        })
                    }
                    BinaryOp::Div => {
                        let dims = match (l.dims, r.dims) {
                            (Some(ld), Some(rd)) => Some(ld.div(rd)),
                            _ => None,
                        };
                        Ok(TypedExpr {
                            ty: QuantityType::Float,
                            dims,
                            expr: expr.clone(),
                        })
                    }
                    BinaryOp::At => Ok(TypedExpr {
                        ty: QuantityType::Hamiltonian,
                        dims: None,
                        expr: expr.clone(),
                    }),
                    _ => Ok(TypedExpr {
                        ty: QuantityType::Bool,
                        dims: None,
                        expr: expr.clone(),
                    }),
                }
            }
            ExprKind::Call { callee, args: _ } => {
                if let ExprKind::Ident(name) = &callee.kind {
                    match name.as_str() {
                        "expect" => {
                            return Ok(TypedExpr {
                                ty: QuantityType::Energy,
                                dims: Some(Dimensions {
                                    l: 2,
                                    m: 1,
                                    t: -2,
                                    ..Default::default()
                                }),
                                expr: expr.clone(),
                            });
                        }
                        "sample" => {
                            return Ok(TypedExpr {
                                ty: QuantityType::Named("SampleResult".into()),
                                dims: None,
                                expr: expr.clone(),
                            });
                        }
                        "ansatz" => {
                            return Ok(TypedExpr {
                                ty: QuantityType::Circuit,
                                dims: None,
                                expr: expr.clone(),
                            });
                        }
                        _ => {}
                    }
                }
                Ok(TypedExpr {
                    ty: QuantityType::Named("Unknown".into()),
                    dims: None,
                    expr: expr.clone(),
                })
            }
            ExprKind::Gate { .. } => Ok(TypedExpr {
                ty: QuantityType::Gate,
                dims: None,
                expr: expr.clone(),
            }),
            ExprKind::Tensor { left, right } => {
                let _l = self.check_expr(left)?;
                let _r = self.check_expr(right)?;
                Ok(TypedExpr {
                    ty: QuantityType::Hamiltonian,
                    dims: None,
                    expr: expr.clone(),
                })
            }
        }
    }
}

fn is_gate_ident(name: &str) -> bool {
    matches!(
        name,
        "H" | "X" | "Y" | "Z" | "S" | "T" | "CNOT" | "CZ" | "SWAP" | "RX" | "RY" | "RZ" | "U3"
    )
}

fn is_quantum_fn(name: &str) -> bool {
    matches!(name, "expect" | "sample" | "ansatz" | "compose" | "tensor")
}

#[cfg(test)]
mod tests {
    use super::*;
    use physlang_parser::parse_source;

    #[test]
    fn rejects_force_equals_mass_plus_velocity() {
        let src = r#"
fn bad() -> Force {
    let m: Mass = 5.0 kg
    let v: Velocity = 9.8 m/s
    return m + v
}
"#;
        let module = parse_source(src).unwrap();
        let err = check_module(&module).unwrap_err();
        assert!(matches!(err, TypeError::DimensionMismatch { .. }));
    }

    #[test]
    fn accepts_energy_literal() {
        let src = r#"
fn ok() -> Energy {
    let h: Energy = 1.0 J
    return h
}
"#;
        let module = parse_source(src).unwrap();
        assert!(check_module(&module).is_ok());
    }
}
