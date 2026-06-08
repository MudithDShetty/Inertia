use physlang_parser::parse_source;
use physlang_types::check_module;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

pub fn diagnostics_for_source(source: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    match parse_source(source) {
        Ok(module) => {
            if let Err(e) = check_module(&module) {
                diags.push(type_error_to_diag(&e));
            }
        }
        Err(e) => {
            diags.push(parse_error_to_diag(&e));
        }
    }
    diags
}

fn parse_error_to_diag(e: &physlang_parser::ParseError) -> Diagnostic {
    match e {
        physlang_parser::ParseError::UnexpectedToken { line, column, message } => Diagnostic {
            line: *line,
            column: *column,
            message: message.clone(),
            severity: DiagnosticSeverity::Error,
        },
        physlang_parser::ParseError::UnexpectedEof => Diagnostic {
            line: 1,
            column: 1,
            message: "unexpected end of file".into(),
            severity: DiagnosticSeverity::Error,
        },
        physlang_parser::ParseError::InvalidNumber(s) => Diagnostic {
            line: 1,
            column: 1,
            message: format!("invalid number: {s}"),
            severity: DiagnosticSeverity::Error,
        },
    }
}

fn type_error_to_diag(e: &physlang_types::TypeError) -> Diagnostic {
    match e {
        physlang_types::TypeError::DimensionMismatch { line, expected, found } => Diagnostic {
            line: *line,
            column: 1,
            message: format!("dimension mismatch: expected {expected}, found {found}"),
            severity: DiagnosticSeverity::Error,
        },
        physlang_types::TypeError::TypeMismatch { line, expected, found } => Diagnostic {
            line: *line,
            column: 1,
            message: format!("type mismatch: expected {expected}, found {found}"),
            severity: DiagnosticSeverity::Error,
        },
        physlang_types::TypeError::UndefinedIdent { line, name } => Diagnostic {
            line: *line,
            column: 1,
            message: format!("undefined identifier '{name}'"),
            severity: DiagnosticSeverity::Error,
        },
        physlang_types::TypeError::Other { line, message } => Diagnostic {
            line: *line,
            column: 1,
            message: message.clone(),
            severity: DiagnosticSeverity::Error,
        },
    }
}
