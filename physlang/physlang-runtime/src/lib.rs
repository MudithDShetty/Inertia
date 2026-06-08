//! PhysicsLang interpreter runtime.

mod exec;
mod value;

pub use exec::{compile_and_run, compile_mir, ExecError, ExecResult, RuntimeOutput};
pub use value::RuntimeValue;
