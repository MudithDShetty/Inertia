//! Quick-fix code actions from diagnostics and cursor context.

use crate::diagnostics::{diagnostics_for_source, DiagnosticSeverity};
use crate::symbols::{collect_symbols, TextEdit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub edits: Vec<TextEdit>,
}

const COMMON_UNITS: &[&str] = &[
    "m", "km", "kg", "g", "s", "A", "K", "mol", "N", "J", "Pa", "W",
];

pub fn code_actions_at(source: &str, line: u32, column: u32) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    let line_text = source
        .lines()
        .nth(line.saturating_sub(1) as usize)
        .unwrap_or("");
    for diag in diagnostics_for_source(source) {
        if diag.line != line || diag.severity != DiagnosticSeverity::Error {
            continue;
        }
        if diag.message.contains("unknown unit") {
            actions.extend(unknown_unit_actions(line, line_text));
        }
        if diag.message.starts_with("dimension mismatch:") {
            if let Some(action) = dimension_mismatch_action(line, line_text, &diag.message) {
                actions.push(action);
            }
        }
        if diag.message.starts_with("undefined identifier") {
            if let Some(action) = undefined_stdlib_hint(line, column, source, &diag.message) {
                actions.push(action);
            }
        }
    }
    actions
}

fn unknown_unit_actions(line: u32, line_text: &str) -> Vec<CodeAction> {
    let mut actions = Vec::new();
    for unit in COMMON_UNITS {
        if line_text.contains(unit) {
            continue;
        }
        actions.push(CodeAction {
            title: format!("Append SI unit '{unit}'"),
            edits: vec![TextEdit {
                line,
                column: line_text.len() as u32 + 1,
                end_column: line_text.len() as u32 + 1,
                new_text: format!(" {unit}"),
            }],
        });
    }
    actions.truncate(6);
    actions
}

fn dimension_mismatch_action(line: u32, _line_text: &str, message: &str) -> Option<CodeAction> {
    let expected = message
        .split("expected ")
        .nth(1)?
        .split(',')
        .next()?
        .trim();
    Some(CodeAction {
        title: format!("Review: expected dimension {expected}"),
        edits: vec![TextEdit {
            line,
            column: 1,
            end_column: 1,
            new_text: format!("// expected dimension: {expected}\n"),
        }],
    })
}

fn undefined_stdlib_hint(
    line: u32,
    column: u32,
    source: &str,
    message: &str,
) -> Option<CodeAction> {
    let name = message
        .strip_prefix("undefined identifier '")?
        .strip_suffix('\'')?;
    let symbols = collect_symbols(source);
    let at = symbols.iter().find(|s| {
        s.line == line && column >= s.column && column <= s.end_column && s.name == name
    })?;
    if at.is_definition {
        return None;
    }
    Some(CodeAction {
        title: format!("'{name}' may be a stdlib symbol — use Go to Definition (F12)"),
        edits: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_mismatch_offers_comment() {
        let src = "fn f() -> Force {\n    let x: Mass = 1.0\n    return x\n}";
        let diags = crate::diagnostics::diagnostics_for_source(src);
        let err_line = diags
            .iter()
            .find(|d| d.message.contains("dimension mismatch"))
            .map(|d| d.line)
            .expect("dimension mismatch diagnostic");
        let actions = code_actions_at(src, err_line, 1);
        assert!(actions.iter().any(|a| a.title.contains("expected dimension")));
    }
}
