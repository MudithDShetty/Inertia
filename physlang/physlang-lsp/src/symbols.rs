//! User-symbol index: completion, find references, rename.

use physlang_parser::{parse_source, Block, Expr, ExprKind, Item, Span, Stmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Parameter,
    Variable,
    QReg,
    Extern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolOccurrence {
    pub name: String,
    pub line: u32,
    pub column: u32,
    pub end_column: u32,
    pub kind: SymbolKind,
    pub is_definition: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub line: u32,
    pub column: u32,
    pub end_column: u32,
    pub new_text: String,
}

fn span_loc(span: &Span) -> (u32, u32, u32) {
    (span.start_line, span.start_col, span.end_col.max(span.start_col + 1))
}

fn push_ident(out: &mut Vec<SymbolOccurrence>, name: &str, span: &Span, kind: SymbolKind, def: bool) {
    let (line, column, end_column) = span_loc(span);
    out.push(SymbolOccurrence {
        name: name.to_string(),
        line,
        column,
        end_column,
        kind,
        is_definition: def,
    });
}

pub fn collect_symbols(source: &str) -> Vec<SymbolOccurrence> {
    let Ok(module) = parse_source(source) else {
        return vec![];
    };
    let mut out = Vec::new();
    for item in &module.items {
        collect_item(item, &mut out);
    }
    out
}

fn collect_item(item: &Item, out: &mut Vec<SymbolOccurrence>) {
    match item {
        Item::Function(f) => {
            push_ident(out, &f.name, &f.span, SymbolKind::Function, true);
            for p in &f.params {
                push_ident(out, &p.name, &p.span, SymbolKind::Parameter, true);
            }
            collect_block(&f.body, out);
        }
        Item::QReg(q) => push_ident(out, &q.name, &q.span, SymbolKind::QReg, true),
        Item::Let(l) => {
            push_ident(out, &l.name, &l.span, SymbolKind::Variable, true);
            collect_expr(&l.value, out);
        }
        Item::Extern(e) => {
            push_ident(out, &e.name, &e.span, SymbolKind::Extern, true);
            for p in &e.params {
                push_ident(out, &p.name, &p.span, SymbolKind::Parameter, true);
            }
        }
    }
}

fn collect_block(block: &Block, out: &mut Vec<SymbolOccurrence>) {
    for stmt in &block.stmts {
        collect_stmt(stmt, out);
    }
}

fn collect_stmt(stmt: &Stmt, out: &mut Vec<SymbolOccurrence>) {
    match stmt {
        Stmt::Let(l) => {
            push_ident(out, &l.name, &l.span, SymbolKind::Variable, true);
            collect_expr(&l.value, out);
        }
        Stmt::Return { value: Some(v), .. } => collect_expr(v, out),
        Stmt::Return { .. } => {}
        Stmt::Expr { expr, .. } => collect_expr(expr, out),
    }
}

fn collect_expr(expr: &Expr, out: &mut Vec<SymbolOccurrence>) {
    match &expr.kind {
        ExprKind::Ident(name) => {
            push_ident(out, name, &expr.span, SymbolKind::Variable, false);
        }
        ExprKind::Unary { expr, .. } => collect_expr(expr, out),
        ExprKind::Binary { left, right, .. } => {
            collect_expr(left, out);
            collect_expr(right, out);
        }
        ExprKind::Call { callee, args, .. } => {
            collect_expr(callee, out);
            for a in args {
                collect_expr(a, out);
            }
        }
        ExprKind::Gate { params, .. } => {
            for p in params {
                collect_expr(p, out);
            }
        }
        ExprKind::Tensor { left, right, .. } => {
            collect_expr(left, out);
            collect_expr(right, out);
        }
        _ => {}
    }
}

pub fn user_completions_for_prefix(source: &str, prefix: &str) -> Vec<(String, SymbolKind)> {
    let p = prefix.trim();
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for sym in collect_symbols(source) {
        if !sym.is_definition {
            continue;
        }
        if !p.is_empty() && !sym.name.to_lowercase().starts_with(&p.to_lowercase()) {
            continue;
        }
        if seen.insert(sym.name.clone()) {
            out.push((sym.name, sym.kind));
        }
    }
    out
}

fn symbol_at(symbols: &[SymbolOccurrence], line: u32, column: u32) -> Option<&SymbolOccurrence> {
    symbols.iter().find(|s| {
        s.line == line && column >= s.column && column <= s.end_column
    })
}

pub fn find_references_at(source: &str, line: u32, column: u32) -> Vec<SourceLocation> {
    let symbols = collect_symbols(source);
    let Some(at) = symbol_at(&symbols, line, column) else {
        return vec![];
    };
    symbols
        .iter()
        .filter(|s| s.name == at.name)
        .map(|s| SourceLocation {
            line: s.line,
            column: s.column,
            end_column: s.end_column,
        })
        .collect()
}

pub fn rename_at(source: &str, line: u32, column: u32, new_name: &str) -> Vec<TextEdit> {
    if new_name.trim().is_empty() || !new_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return vec![];
    }
    let symbols = collect_symbols(source);
    let Some(at) = symbol_at(&symbols, line, column) else {
        return vec![];
    };
    symbols
        .iter()
        .filter(|s| s.name == at.name)
        .map(|s| TextEdit {
            line: s.line,
            column: s.column,
            end_column: s.end_column,
            new_text: new_name.to_string(),
        })
        .collect()
}

/// Go to the definition site for the symbol at `(line, column)`.
pub fn definition_at(source: &str, line: u32, column: u32) -> Option<SourceLocation> {
    let symbols = collect_symbols(source);
    let at = symbol_at(&symbols, line, column)?;
    symbols
        .iter()
        .find(|s| s.name == at.name && s.is_definition)
        .map(|s| SourceLocation {
            line: s.line,
            column: s.column,
            end_column: s.end_column,
        })
}

/// Symbol name under the cursor, if any.
pub fn symbol_name_at(source: &str, line: u32, column: u32) -> Option<String> {
    let symbols = collect_symbols(source);
    symbol_at(&symbols, line, column)
        .map(|s| s.name.clone())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionLocation {
    pub line: u32,
    pub column: u32,
    pub end_column: u32,
    /// Absolute path when definition is in another file (e.g. stdlib).
    pub file: Option<String>,
}

/// Go to definition in the current file, then stdlib index fallback.
pub fn definition_at_with_stdlib(
    source: &str,
    line: u32,
    column: u32,
    stdlib_index: &std::collections::HashMap<String, crate::stdlib::StdlibSymbol>,
) -> Option<DefinitionLocation> {
    if let Some(loc) = definition_at(source, line, column) {
        return Some(DefinitionLocation {
            line: loc.line,
            column: loc.column,
            end_column: loc.end_column,
            file: None,
        });
    }
    let name = symbol_name_at(source, line, column)?;
    let sym = crate::stdlib::stdlib_definition_at(stdlib_index, &name)?;
    Some(DefinitionLocation {
        line: sym.line,
        column: sym.column,
        end_column: sym.end_column,
        file: Some(sym.file.display().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"fn energy(theta: Float) -> Energy {
    let x = theta;
    return x;
}
"#;

    #[test]
    fn finds_references_for_param() {
        let refs = find_references_at(SAMPLE, 1, 11);
        assert!(refs.len() >= 2);
    }

    #[test]
    fn rename_theta() {
        let edits = rename_at(SAMPLE, 1, 11, "angle");
        assert!(edits.len() >= 2);
        assert!(edits.iter().all(|e| e.new_text == "angle"));
    }

    #[test]
    fn user_completion_includes_fn_name() {
        let items = user_completions_for_prefix(SAMPLE, "en");
        assert!(items.iter().any(|(n, _)| n == "energy"));
    }

    #[test]
    fn go_to_definition_param() {
        let def = definition_at(SAMPLE, 1, 11).expect("def");
        assert_eq!(def.line, 1);
        assert_eq!(def.column, 11);
    }
}
