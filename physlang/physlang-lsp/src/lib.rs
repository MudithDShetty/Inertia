//! PhysicsLang Language Server Protocol helpers.

mod completion;
mod diagnostics;
mod hover;

pub use completion::{completions_for_prefix, CompletionItem, CompletionKind};
pub use diagnostics::{diagnostics_for_source, Diagnostic, DiagnosticSeverity};
pub use hover::{hover_for_position, HoverInfo};
