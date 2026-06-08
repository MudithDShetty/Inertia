//! PhysicsLang type checker and SI unit system.

mod check;
mod units;

pub use check::{check_module, TypedExpr, TypedFunction, TypedItem, TypedModule, TypeError, TypeResult};
pub use units::{Dimensions, QuantityType, UnitRegistry, UnitSpec};
