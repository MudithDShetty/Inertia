use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseError {
    #[error("unexpected token at line {line}, column {column}: {message}")]
    UnexpectedToken {
        line: u32,
        column: u32,
        message: String,
    },
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("invalid numeric literal: {0}")]
    InvalidNumber(String),
}

pub type ParseResult<T> = Result<T, ParseError>;
