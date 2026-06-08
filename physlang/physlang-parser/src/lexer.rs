use crate::error::{ParseError, ParseResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Fn,
    Let,
    Return,
    True,
    False,
    Qreg,
    Extern,
    If,
    Else,
    // Types / units
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    At,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Semicolon,
    Dot,
    Arrow,
    // Special
    Eof,
}

#[derive(Debug, Clone)]
pub struct TokenWithSpan {
    pub token: Token,
    pub line: u32,
    pub column: u32,
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: u32,
    column: u32,
    offset: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
            offset: 0,
        }
    }

    pub fn tokenize(&mut self) -> ParseResult<Vec<TokenWithSpan>> {
        let mut tokens = Vec::new();
        loop {
            let t = self.next_token()?;
            let is_eof = matches!(t.token, Token::Eof);
            tokens.push(t);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> ParseResult<TokenWithSpan> {
        self.skip_whitespace_and_comments();
        let line = self.line;
        let column = self.column;

        let Some((_, ch)) = self.peek_char() else {
            return Ok(TokenWithSpan {
                token: Token::Eof,
                line,
                column,
            });
        };

        let token = match ch {
            '(' => {
                self.advance();
                Token::LParen
            }
            ')' => {
                self.advance();
                Token::RParen
            }
            '[' => {
                self.advance();
                Token::LBracket
            }
            ']' => {
                self.advance();
                Token::RBracket
            }
            '{' => {
                self.advance();
                Token::LBrace
            }
            '}' => {
                self.advance();
                Token::RBrace
            }
            ',' => {
                self.advance();
                Token::Comma
            }
            ':' => {
                self.advance();
                Token::Colon
            }
            ';' => {
                self.advance();
                Token::Semicolon
            }
            '.' => {
                self.advance();
                Token::Dot
            }
            '+' => {
                self.advance();
                Token::Plus
            }
            '*' => {
                self.advance();
                Token::Star
            }
            '/' => {
                self.advance();
                if self.peek_char().map(|(_, c)| c) == Some('/') {
                    self.skip_line_comment();
                    return self.next_token();
                }
                Token::Slash
            }
            '^' => {
                self.advance();
                Token::Caret
            }
            '@' => {
                self.advance();
                Token::At
            }
            '!' => {
                self.advance();
                if self.peek_char().map(|(_, c)| c) == Some('=') {
                    self.advance();
                    Token::Ne
                } else {
                    Token::Bang
                }
            }
            '-' => {
                self.advance();
                if self.peek_char().map(|(_, c)| c) == Some('>') {
                    self.advance();
                    Token::Arrow
                } else if self.peek_char().map(|(_, c)| c.is_ascii_digit()) == Some(true) {
                    return self.read_number(true, line, column);
                } else {
                    Token::Minus
                }
            }
            '=' => {
                self.advance();
                if self.peek_char().map(|(_, c)| c) == Some('=') {
                    self.advance();
                    Token::EqEq
                } else {
                    Token::Eq
                }
            }
            '<' => {
                self.advance();
                if self.peek_char().map(|(_, c)| c) == Some('=') {
                    self.advance();
                    Token::Le
                } else {
                    Token::Lt
                }
            }
            '>' => {
                self.advance();
                if self.peek_char().map(|(_, c)| c) == Some('=') {
                    self.advance();
                    Token::Ge
                } else {
                    Token::Gt
                }
            }
            '"' => self.read_string()?,
            c if c.is_ascii_digit() => return self.read_number(false, line, column),
            c if is_ident_start(c) => self.read_ident_or_keyword()?,
            _ => {
                return Err(ParseError::UnexpectedToken {
                    line,
                    column,
                    message: format!("unexpected character '{ch}'"),
                });
            }
        };

        Ok(TokenWithSpan {
            token,
            line,
            column,
        })
    }

    fn read_number(
        &mut self,
        negative: bool,
        line: u32,
        column: u32,
    ) -> ParseResult<TokenWithSpan> {
        let start = self.current_pos();
        if negative {
            self.advance();
        }
        while self.peek_char().map(|(_, c)| c.is_ascii_digit()) == Some(true) {
            self.advance();
        }
        let mut is_float = false;
        if self.peek_char().map(|(_, c)| c) == Some('.') {
            is_float = true;
            self.advance();
            while self.peek_char().map(|(_, c)| c.is_ascii_digit()) == Some(true) {
                self.advance();
            }
        }
        if self.peek_char().map(|(_, c)| c) == Some('e') || self.peek_char().map(|(_, c)| c) == Some('E')
        {
            is_float = true;
            self.advance();
            if self.peek_char().map(|(_, c)| c) == Some('+') || self.peek_char().map(|(_, c)| c) == Some('-') {
                self.advance();
            }
            if self.peek_char().map(|(_, c)| c.is_ascii_digit()) != Some(true) {
                return Err(ParseError::InvalidNumber(
                    self.slice(start, self.current_pos()).to_string(),
                ));
            }
            while self.peek_char().map(|(_, c)| c.is_ascii_digit()) == Some(true) {
                self.advance();
            }
        }
        let text = self.slice(start, self.current_pos());
        let token = if is_float {
            let v: f64 = text.parse().map_err(|_| ParseError::InvalidNumber(text.to_string()))?;
            Token::Float(if negative { -v } else { v })
        } else {
            let v: i64 = text.parse().map_err(|_| ParseError::InvalidNumber(text.to_string()))?;
            Token::Int(if negative { -v } else { v })
        };
        Ok(TokenWithSpan {
            token,
            line,
            column,
        })
    }

    fn read_string(&mut self) -> ParseResult<Token> {
        self.advance(); // opening quote
        let start = self.current_pos();
        while let Some((_, c)) = self.peek_char() {
            if c == '"' {
                let s = self.slice(start, self.current_pos()).to_string();
                self.advance();
                return Ok(Token::String(s));
            }
            if c == '\\' {
                self.advance();
                self.advance();
            } else {
                self.advance();
            }
        }
        Err(ParseError::UnexpectedEof)
    }

    fn read_ident_or_keyword(&mut self) -> ParseResult<Token> {
        let start = self.current_pos();
        self.advance();
        while self
            .peek_char()
            .map(|(_, c)| is_ident_continue(c))
            .unwrap_or(false)
        {
            self.advance();
        }
        let text = self.slice(start, self.current_pos());
        let token = match text {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "return" => Token::Return,
            "true" => Token::True,
            "false" => Token::False,
            "qreg" => Token::Qreg,
            "extern" => Token::Extern,
            "if" => Token::If,
            "else" => Token::Else,
            _ => Token::Ident(text.to_string()),
        };
        Ok(token)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_char() {
                Some((_, c)) if c.is_whitespace() => {
                    if c == '\n' {
                        self.line += 1;
                        self.column = 0;
                    }
                    self.advance();
                }
                Some((_, '/')) if self.peek_next_char() == Some('/') => {
                    self.advance();
                    self.advance();
                    self.skip_line_comment();
                }
                _ => break,
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some((_, c)) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn peek_char(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }

    fn peek_next_char(&mut self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().map(|(_, c)| c)
    }

    fn advance(&mut self) {
        if let Some((idx, c)) = self.chars.next() {
            self.offset = idx + c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    fn current_pos(&self) -> usize {
        self.chars.clone().peek().map(|(i, _)| *i).unwrap_or(self.source.len())
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        &self.source[start..end.min(self.source.len())]
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
