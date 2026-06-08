//! PhysicsLang AST and parser for `.phys` source files.

mod ast;
mod error;
mod lexer;
mod parse;

pub use ast::*;
pub use error::{ParseError, ParseResult};
pub use parse::parse_source;
