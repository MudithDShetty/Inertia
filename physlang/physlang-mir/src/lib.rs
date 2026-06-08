//! PhysicsLang MIR lowering and autodiff transforms.

mod autodiff;
mod lower;

pub use autodiff::{autodiff_function, DiffMode, DiffProgram};
pub use lower::{lower_module, MirBinaryOp, MirFunction, MirModule, MirOp, MirStmt, MirUnaryOp, MirValue};
