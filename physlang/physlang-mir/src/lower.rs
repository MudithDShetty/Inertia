use physlang_parser::{BinaryOp, Expr, ExprKind, FunctionDef, Item, Stmt, UnaryOp};
use physlang_types::{TypedFunction, TypedItem, TypedModule};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
    pub qregs: Vec<(String, u32)>,
    pub globals: Vec<(String, MirValue)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<String>,
    pub is_differentiable: bool,
    pub python_import: Option<String>,
    pub body: Vec<MirStmt>,
    pub ret_ty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirStmt {
    Let { name: String, value: MirValue },
    Return { value: Option<MirValue> },
    Expr(MirValue),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Quantity { value: f64, unit: String },
    Ident(String),
    Unary { op: MirUnaryOp, expr: Box<MirValue> },
    Binary {
        op: MirBinaryOp,
        left: Box<MirValue>,
        right: Box<MirValue>,
    },
    Call { name: String, args: Vec<MirValue> },
    Gate {
        name: String,
        targets: Vec<u32>,
        params: Vec<MirValue>,
    },
    Tensor {
        left: Box<MirValue>,
        right: Box<MirValue>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MirUnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    At,
}

pub fn lower_module(typed: &TypedModule) -> MirModule {
    let mut functions = Vec::new();
    let mut qregs = Vec::new();
    let mut globals = Vec::new();

    for item in &typed.items {
        match item {
            TypedItem::Function(tf) => functions.push(lower_function(tf)),
            TypedItem::QReg { name, size } => qregs.push((name.clone(), *size)),
            TypedItem::Let { name, value, .. } => {
                globals.push((name.clone(), lower_expr(&value.expr)));
            }
        }
    }

    MirModule {
        functions,
        qregs,
        globals,
    }
}

fn lower_function(tf: &TypedFunction) -> MirFunction {
    MirFunction {
        name: tf.def.name.clone(),
        params: tf.def.params.iter().map(|p| p.name.clone()).collect(),
        is_differentiable: tf.is_differentiable,
        python_import: tf.python_import.clone(),
        body: tf
            .def
            .body
            .stmts
            .iter()
            .map(lower_stmt)
            .collect(),
        ret_ty: format!("{:?}", tf.ret_ty),
    }
}

fn lower_stmt(stmt: &Stmt) -> MirStmt {
    match stmt {
        Stmt::Let(l) => MirStmt::Let {
            name: l.name.clone(),
            value: lower_expr(&l.value),
        },
        Stmt::Return { value, .. } => MirStmt::Return {
            value: value.as_ref().map(lower_expr),
        },
        Stmt::Expr { expr, .. } => MirStmt::Expr(lower_expr(expr)),
    }
}

fn lower_expr(expr: &Expr) -> MirValue {
    match &expr.kind {
        ExprKind::Int(v) => MirValue::Int(*v),
        ExprKind::Float(v) => MirValue::Float(*v),
        ExprKind::Bool(v) => MirValue::Bool(*v),
        ExprKind::String(s) => MirValue::String(s.clone()),
        ExprKind::Quantity { value, unit } => MirValue::Quantity {
            value: *value,
            unit: unit
                .factors
                .iter()
                .map(|f| {
                    if f.power == 1 {
                        f.ident.clone()
                    } else {
                        format!("{}^{}", f.ident, f.power)
                    }
                })
                .collect::<Vec<_>>()
                .join("*"),
        },
        ExprKind::Ident(s) => MirValue::Ident(s.clone()),
        ExprKind::Unary { op, expr } => MirValue::Unary {
            op: match op {
                UnaryOp::Neg => MirUnaryOp::Neg,
                UnaryOp::Not => MirUnaryOp::Not,
            },
            expr: Box::new(lower_expr(expr)),
        },
        ExprKind::Binary { op, left, right } => MirValue::Binary {
            op: match op {
                BinaryOp::Add => MirBinaryOp::Add,
                BinaryOp::Sub => MirBinaryOp::Sub,
                BinaryOp::Mul => MirBinaryOp::Mul,
                BinaryOp::Div => MirBinaryOp::Div,
                BinaryOp::At => MirBinaryOp::At,
                _ => MirBinaryOp::Add,
            },
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },
        ExprKind::Call { callee, args } => {
            let name = match &callee.kind {
                ExprKind::Ident(s) => s.clone(),
                _ => "unknown".into(),
            };
            MirValue::Call {
                name,
                args: args.iter().map(lower_expr).collect(),
            }
        }
        ExprKind::Gate {
            name,
            targets,
            params,
        } => MirValue::Gate {
            name: name.clone(),
            targets: targets.clone(),
            params: params.iter().map(lower_expr).collect(),
        },
        ExprKind::Tensor { left, right } => MirValue::Tensor {
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirOp {
    LoadConst(MirValue),
    Store(String, MirValue),
    Return(MirValue),
}
