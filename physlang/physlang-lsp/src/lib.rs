//! PhysicsLang Language Server Protocol helpers.

mod code_actions;
mod completion;
mod diagnostics;
mod hover;
mod stdlib;
mod stdlib_docs;
mod symbols;

pub use code_actions::{code_actions_at, CodeAction};
pub use completion::{completions_for_prefix, CompletionItem, CompletionKind};
pub use diagnostics::{diagnostics_for_source, Diagnostic, DiagnosticSeverity};
pub use hover::{hover_for_position, HoverInfo};
pub use stdlib::{find_stdlib_dir, index_stdlib_dir, StdlibSymbol};
pub use stdlib_docs::{doc_lines_before, generate_stdlib_markdown};
pub use symbols::{
    collect_symbols, definition_at, definition_at_with_stdlib, find_references_at, rename_at,
    user_completions_for_prefix, DefinitionLocation, SourceLocation, SymbolKind, SymbolOccurrence,
    TextEdit,
};
