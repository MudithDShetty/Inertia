use crate::ast::*;
use crate::error::{ParseError, ParseResult};
use crate::lexer::{Lexer, Token, TokenWithSpan};

pub fn parse_source(source: &str) -> ParseResult<Module> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_module()
}

struct Parser {
    tokens: Vec<TokenWithSpan>,
    pos: usize,
}

impl Parser {
    fn parse_module(&mut self) -> ParseResult<Module> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }
        Ok(Module { items })
    }

    fn parse_item(&mut self) -> ParseResult<Item> {
        if self.check(Token::At) {
            let attrs = self.parse_attributes()?;
            if self.check(Token::Fn) {
                return Ok(Item::Function(self.parse_function(attrs)?));
            }
            if self.check(Token::Extern) {
                return Ok(Item::Extern(self.parse_extern(attrs)?));
            }
        }
        if self.check(Token::Fn) {
            return Ok(Item::Function(self.parse_function(vec![])?));
        }
        if self.check(Token::Qreg) {
            return Ok(Item::QReg(self.parse_qreg()?));
        }
        if self.check(Token::Let) {
            let let_stmt = self.parse_let_stmt()?;
            self.consume_semicolon_optional();
            return Ok(Item::Let(let_stmt));
        }
        if self.check(Token::Extern) {
            return Ok(Item::Extern(self.parse_extern(vec![])?));
        }
        Err(self.error("expected fn, qreg, let, or extern"))
    }

    fn parse_attributes(&mut self) -> ParseResult<Vec<Attribute>> {
        let mut attrs = Vec::new();
        while self.check(Token::At) {
            let span = self.span();
            self.advance(); // @
            let mut name = self.expect_ident()?;
            while self.check(Token::Dot) {
                self.advance();
                name.push('.');
                name.push_str(&self.expect_ident()?);
            }
            let mut args = Vec::new();
            if self.check(Token::LParen) {
                self.advance();
                if !self.check(Token::RParen) {
                    loop {
                        args.push(self.expect_string_or_ident()?);
                        if self.check(Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RParen)?;
            }
            attrs.push(Attribute { name, args, span });
        }
        Ok(attrs)
    }

    fn parse_function(&mut self, attrs: Vec<Attribute>) -> ParseResult<FunctionDef> {
        let span = self.span();
        self.expect(Token::Fn)?;
        let name = self.expect_ident()?;
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if !self.check(Token::RParen) {
            loop {
                let pspan = self.span();
                let pname = self.expect_ident()?;
                self.expect(Token::Colon)?;
                let pty = self.parse_type_expr()?;
                params.push(Param {
                    name: pname,
                    ty: pty,
                    span: pspan,
                });
                if self.check(Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen)?;
        let ret_type = if self.check(Token::Arrow) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(FunctionDef {
            name,
            attrs,
            params,
            ret_type,
            body,
            span,
        })
    }

    fn parse_extern(&mut self, _attrs: Vec<Attribute>) -> ParseResult<ExternDecl> {
        let span = self.span();
        self.expect(Token::Extern)?;
        let lang = self.expect_string_or_ident()?;
        self.expect(Token::Fn)?;
        let name = self.expect_ident()?;
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if !self.check(Token::RParen) {
            loop {
                let pspan = self.span();
                let pname = self.expect_ident()?;
                self.expect(Token::Colon)?;
                let pty = self.parse_type_expr()?;
                params.push(Param {
                    name: pname,
                    ty: pty,
                    span: pspan,
                });
                if self.check(Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen)?;
        let ret_type = if self.check(Token::Arrow) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.consume_semicolon_optional();
        Ok(ExternDecl {
            lang,
            name,
            params,
            ret_type,
            span,
        })
    }

    fn parse_qreg(&mut self) -> ParseResult<QRegDecl> {
        let span = self.span();
        self.expect(Token::Qreg)?;
        let name = self.expect_ident()?;
        self.expect(Token::LBracket)?;
        let size = self.expect_int()? as u32;
        self.expect(Token::RBracket)?;
        self.consume_semicolon_optional();
        Ok(QRegDecl { name, size, span })
    }

    fn parse_let_stmt(&mut self) -> ParseResult<LetStmt> {
        let span = self.span();
        self.expect(Token::Let)?;
        let name = self.expect_ident()?;
        let ty = if self.check(Token::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(LetStmt {
            name,
            ty,
            value,
            span,
        })
    }

    fn parse_block(&mut self) -> ParseResult<Block> {
        let span = self.span();
        self.expect(Token::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(Token::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Block { stmts, span })
    }

    fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        if self.check(Token::Let) {
            let let_stmt = self.parse_let_stmt()?;
            self.consume_semicolon_optional();
            return Ok(Stmt::Let(let_stmt));
        }
        if self.check(Token::Return) {
            let span = self.span();
            self.advance();
            let value = if self.check(Token::Semicolon) || self.check(Token::RBrace) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.consume_semicolon_optional();
            return Ok(Stmt::Return { value, span });
        }
        let span = self.span();
        let expr = self.parse_expr()?;
        self.consume_semicolon_optional();
        Ok(Stmt::Expr { expr, span })
    }

    fn parse_type_expr(&mut self) -> ParseResult<TypeExpr> {
        let span = self.span();
        let name = self.expect_ident()?;
        if self.check(Token::LBracket) {
            self.advance();
            let len = self.expect_int()? as u32;
            self.expect(Token::RBracket)?;
            let elem = TypeExpr {
                kind: TypeKind::Named(name),
                span: span.clone(),
            };
            return Ok(TypeExpr {
                kind: TypeKind::Array {
                    elem: Box::new(elem),
                    len,
                },
                span,
            });
        }
        Ok(TypeExpr {
            kind: TypeKind::Named(name),
            span,
        })
    }

    fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_and()?;
        while self.check(Token::OrOr) {
            let span = self.span();
            self.advance();
            let right = self.parse_and()?;
            left = Expr {
                kind: ExprKind::Binary {
                    op: BinaryOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_equality()?;
        while self.check(Token::AndAnd) {
            let span = self.span();
            self.advance();
            let right = self.parse_equality()?;
            left = Expr {
                kind: ExprKind::Binary {
                    op: BinaryOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_comparison()?;
        while self.check(Token::EqEq) || self.check(Token::Ne) {
            let span = self.span();
            let op = if self.check(Token::EqEq) {
                self.advance();
                BinaryOp::Eq
            } else {
                self.advance();
                BinaryOp::Ne
            };
            let right = self.parse_comparison()?;
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_additive()?;
        while self.check(Token::Lt)
            || self.check(Token::Le)
            || self.check(Token::Gt)
            || self.check(Token::Ge)
        {
            let span = self.span();
            let op = match self.peek_token() {
                Token::Lt => {
                    self.advance();
                    BinaryOp::Lt
                }
                Token::Le => {
                    self.advance();
                    BinaryOp::Le
                }
                Token::Gt => {
                    self.advance();
                    BinaryOp::Gt
                }
                Token::Ge => {
                    self.advance();
                    BinaryOp::Ge
                }
                _ => unreachable!(),
            };
            let right = self.parse_additive()?;
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_multiplicative()?;
        while self.check(Token::Plus) || self.check(Token::Minus) {
            let span = self.span();
            let op = if self.check(Token::Plus) {
                self.advance();
                BinaryOp::Add
            } else {
                self.advance();
                BinaryOp::Sub
            };
            let right = self.parse_multiplicative()?;
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_tensor()?;
        while self.check(Token::Star) || self.check(Token::Slash) {
            let span = self.span();
            let op = if self.check(Token::Star) {
                self.advance();
                BinaryOp::Mul
            } else {
                self.advance();
                BinaryOp::Div
            };
            let right = self.parse_tensor()?;
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_tensor(&mut self) -> ParseResult<Expr> {
        let mut left = self.parse_unary()?;
        while self.check(Token::At) {
            let span = self.span();
            self.advance();
            let right = self.parse_unary()?;
            left = Expr {
                kind: ExprKind::Tensor {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> ParseResult<Expr> {
        if self.check(Token::Minus) {
            let span = self.span();
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                },
                span,
            });
        }
        if self.check(Token::Bang) {
            let span = self.span();
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                },
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.check(Token::LParen) {
                let span = self.span();
                self.advance();
                let mut args = Vec::new();
                if !self.check(Token::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if self.check(Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RParen)?;
                // Gate call: H(0), RX(theta, 0), CNOT(0, 1)
                if let ExprKind::Ident(name) = &expr.kind {
                    if is_gate_name(name) && args.iter().all(is_target_or_param) {
                        let (params, targets) = split_gate_args(&args);
                        expr = Expr {
                            kind: ExprKind::Gate {
                                name: name.clone(),
                                targets,
                                params,
                            },
                            span,
                        };
                        continue;
                    }
                }
                expr = Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(expr),
                        args,
                    },
                    span,
                };
            } else if is_unit_literal_suffix(self.peek_token()) {
                // quantity literal: 9.8 m/s, 1.0 J, 5.0 kg
                if matches!(expr.kind, ExprKind::Float(_) | ExprKind::Int(_)) {
                    expr = self.parse_quantity_suffix(expr)?;
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_quantity_suffix(&mut self, value_expr: Expr) -> ParseResult<Expr> {
        let span = value_expr.span.clone();
        let value = match &value_expr.kind {
            ExprKind::Float(v) => *v,
            ExprKind::Int(v) => *v as f64,
            _ => {
                return Err(ParseError::UnexpectedToken {
                    line: span.start_line,
                    column: span.start_col,
                    message: "expected numeric value before unit".into(),
                });
            }
        };
        let unit = self.parse_unit_expr()?;
        Ok(Expr {
            kind: ExprKind::Quantity { value, unit },
            span,
        })
    }

    fn parse_unit_expr(&mut self) -> ParseResult<UnitExpr> {
        let span = self.span();
        let mut factors = Vec::new();
        let mut after_slash = false;

        loop {
            let ident = self.expect_ident()?;
            let mut power = 1i32;
            if self.check(Token::Caret) {
                self.advance();
                power = self.expect_int()? as i32;
            }
            if after_slash {
                power = -power;
                after_slash = false;
            }
            factors.push(UnitFactor { ident, power });

            if self.check(Token::Slash) {
                self.advance();
                after_slash = true;
                continue;
            }
            if self.check(Token::Star) {
                self.advance();
                continue;
            }
            break;
        }
        Ok(UnitExpr { factors, span })
    }

    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let span = self.span();
        match self.peek_token() {
            Token::Int(v) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Int(v),
                    span,
                })
            }
            Token::Float(v) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Float(v),
                    span,
                })
            }
            Token::True => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(true),
                    span,
                })
            }
            Token::False => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Bool(false),
                    span,
                })
            }
            Token::String(s) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::String(s),
                    span,
                })
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expr {
                    kind: ExprKind::Ident(name),
                    span,
                })
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            _ => Err(self.error("expected expression")),
        }
    }

    fn expect(&mut self, token: Token) -> ParseResult<()> {
        if self.check(token.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("expected token {:?}", self.peek_token())))
        }
    }

    fn expect_ident(&mut self) -> ParseResult<String> {
        if let Token::Ident(s) = self.peek_token() {
            self.advance();
            Ok(s)
        } else {
            Err(self.error("expected identifier"))
        }
    }

    fn expect_int(&mut self) -> ParseResult<i64> {
        if let Token::Int(v) = self.peek_token() {
            self.advance();
            Ok(v)
        } else {
            Err(self.error("expected integer"))
        }
    }

    fn expect_string_or_ident(&mut self) -> ParseResult<String> {
        match self.peek_token() {
            Token::Ident(s) | Token::String(s) => {
                self.advance();
                Ok(s)
            }
            _ => Err(self.error("expected string or identifier")),
        }
    }

    fn consume_semicolon_optional(&mut self) {
        if self.check(Token::Semicolon) {
            self.advance();
        }
    }

    fn check(&self, token: Token) -> bool {
        !self.is_at_end() && self.tokens[self.pos].token == token
    }

    fn is_at_end(&self) -> bool {
        matches!(self.tokens.get(self.pos).map(|t| &t.token), Some(Token::Eof) | None)
    }

    fn peek_token(&self) -> Token {
        self.tokens
            .get(self.pos)
            .map(|t| t.token.clone())
            .unwrap_or(Token::Eof)
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn span(&self) -> Span {
        let t = &self.tokens[self.pos];
        Span {
            start_line: t.line,
            start_col: t.column,
            end_line: t.line,
            end_col: t.column,
        }
    }

    fn error(&self, message: &str) -> ParseError {
        let t = self.tokens.get(self.pos).or_else(|| self.tokens.last());
        if let Some(t) = t {
            ParseError::UnexpectedToken {
                line: t.line,
                column: t.column,
                message: message.to_string(),
            }
        } else {
            ParseError::UnexpectedEof
        }
    }
}

fn is_gate_name(name: &str) -> bool {
    matches!(
        name,
        "H" | "X" | "Y" | "Z" | "S" | "T" | "CNOT" | "CZ" | "SWAP" | "RX" | "RY" | "RZ" | "U3"
    )
}

fn is_target_or_param(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Ident(_)
    )
}

fn split_gate_args(args: &[Expr]) -> (Vec<Expr>, Vec<u32>) {
    let mut params = Vec::new();
    let mut targets = Vec::new();
    for arg in args {
        match &arg.kind {
            ExprKind::Int(i) => targets.push(*i as u32),
            _ => params.push(arg.clone()),
        }
    }
    (params, targets)
}

fn is_unit_literal_suffix(token: Token) -> bool {
    matches!(token, Token::Ident(_))
}

fn is_unit_start(token: Token) -> bool {
    matches!(token, Token::Ident(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hello_fn() {
        let src = r#"
fn main() -> Int {
    return 42
}
"#;
        let m = parse_source(src).unwrap();
        assert_eq!(m.items.len(), 1);
    }

    #[test]
    fn parse_qreg_and_hamiltonian() {
        let src = r#"
qreg q[2]
let H = X(0) @ X(1) + Z(0)
"#;
        let m = parse_source(src).unwrap();
        assert_eq!(m.items.len(), 2);
    }
}
