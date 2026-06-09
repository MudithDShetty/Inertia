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

/// Default SI unit literal suffix for a named physics type in type annotations.
const TYPE_DEFAULT_UNITS: &[(&str, &str)] = &[
    ("Mass", "kg"),
    ("Velocity", "m/s"),
    ("Force", "N"),
    ("Energy", "J"),
    ("Action", "J*s"),
    ("Pressure", "Pa"),
    ("Power", "W"),
    ("Current", "A"),
    ("Temperature", "K"),
    ("Amount", "mol"),
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
    // Offer SI unit suffix when `let name: PhysicsType = bare_number` (type checker may not error yet).
    for action in unit_suffix_actions(line, line_text) {
        if !actions.iter().any(|a| a.title == action.title) {
            actions.push(action);
        }
    }
    actions
}

fn default_unit_for_type(type_name: &str) -> Option<&'static str> {
    TYPE_DEFAULT_UNITS
        .iter()
        .find(|(t, _)| *t == type_name)
        .map(|(_, u)| *u)
}

fn is_bare_numeric_literal(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        && s.chars().any(|c| c.is_ascii_digit())
}

/// Column (1-based) immediately after a bare numeric literal following `=`.
fn bare_number_end_column(line: &str) -> Option<u32> {
    let eq = line.find('=')?;
    let mut i = eq + 1;
    let bytes = line.as_bytes();
    while i < line.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let start = i;
    while i < line.len() {
        let c = bytes[i];
        if c.is_ascii_digit() || matches!(c, b'.' | b'-' | b'+' | b'e' | b'E') {
            i += 1;
        } else if c == b'/' || c.is_ascii_alphabetic() {
            return None;
        } else {
            break;
        }
    }
    if i == start {
        return None;
    }
    Some((i + 1) as u32)
}

fn unit_suffix_actions(line: u32, line_text: &str) -> Vec<CodeAction> {
    let code = line_text.split("//").next().unwrap_or(line_text).trim();
    if !code.starts_with("let ") || !code.contains(':') {
        return Vec::new();
    }
    let Some(eq_idx) = code.find('=') else {
        return Vec::new();
    };
    let lhs = code[..eq_idx].trim();
    let rhs = code[eq_idx + 1..].trim();
    if !is_bare_numeric_literal(rhs) {
        return Vec::new();
    }
    let Some(colon_idx) = lhs.rfind(':') else {
        return Vec::new();
    };
    let type_name = lhs[colon_idx + 1..].trim();
    let Some(unit) = default_unit_for_type(type_name) else {
        return Vec::new();
    };
    let Some(column) = bare_number_end_column(line_text) else {
        return Vec::new();
    };
    vec![CodeAction {
        title: format!("Add unit suffix `{unit}` for {type_name}"),
        edits: vec![TextEdit {
            line,
            column,
            end_column: column,
            new_text: format!(" {unit}"),
        }],
    }]
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

    #[test]
    fn bare_float_on_typed_let_offers_unit_suffix() {
        let src = "fn main() -> Int {\n    let v: Velocity = 9.8\n    return 0\n}";
        let actions = code_actions_at(src, 2, 10);
        let suffix = actions
            .iter()
            .find(|a| a.title.contains("Add unit suffix"))
            .expect("unit suffix action");
        assert!(suffix.title.contains("m/s"));
        assert_eq!(suffix.edits[0].new_text, " m/s");
    }
}
