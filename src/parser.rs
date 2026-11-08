//! Recursive-descent parser for the Phase 1 language subset.

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::lexer::{Span, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    file: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: &str) -> Self {
        Parser {
            tokens,
            pos: 0,
            file: file.to_string(),
        }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn advance(&mut self) -> TokenKind {
        let kind = self.tokens[self.pos].kind.clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        kind
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn expect(&mut self, kind: &TokenKind, what: &str) -> Result<Span, Diagnostic> {
        if self.at(kind) {
            let span = self.peek_span();
            self.advance();
            Ok(span)
        } else {
            Err(self.err_here(&format!("Expected {}, found {:?}", what, self.peek())))
        }
    }

    fn err_here(&self, message: &str) -> Diagnostic {
        Diagnostic::error("PARSE_ERROR", &self.file, self.peek_span(), message)
    }

    pub fn parse_module(&mut self) -> Result<Module, Diagnostic> {
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut functions = Vec::new();
        while !self.at(&TokenKind::Eof) {
            if self.at(&TokenKind::Struct) {
                structs.push(self.parse_struct()?);
            } else if self.at(&TokenKind::Enum) {
                enums.push(self.parse_enum()?);
            } else if self.at(&TokenKind::Function) {
                functions.push(self.parse_function()?);
            } else {
                return Err(self.err_here("Expected a `struct`, `enum`, or `function` declaration"));
            }
        }
        Ok(Module {
            structs,
            enums,
            functions,
        })
    }

    fn parse_struct(&mut self) -> Result<StructDecl, Diagnostic> {
        let span = self.expect(&TokenKind::Struct, "`struct`")?;
        let name = match self.advance() {
            TokenKind::Identifier(name) => name,
            _ => return Err(self.err_here("Expected struct name")),
        };
        self.expect(&TokenKind::LBrace, "`{`")?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let fspan = self.peek_span();
            let fname = match self.advance() {
                TokenKind::Identifier(n) => n,
                _ => return Err(self.err_here("Expected field name")),
            };
            self.expect(&TokenKind::Colon, "`:`")?;
            let ty = self.parse_type()?;
            fields.push(FieldDecl {
                name: fname,
                ty,
                span: fspan,
            });
            if self.at(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(StructDecl { name, fields, span })
    }

    fn parse_enum(&mut self) -> Result<EnumDecl, Diagnostic> {
        let span = self.expect(&TokenKind::Enum, "`enum`")?;
        let name = match self.advance() {
            TokenKind::Identifier(name) => name,
            _ => return Err(self.err_here("Expected enum name")),
        };
        self.expect(&TokenKind::LBrace, "`{`")?;
        let mut variants = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let vspan = self.peek_span();
            let vname = match self.advance() {
                TokenKind::Identifier(n) => n,
                _ => return Err(self.err_here("Expected variant name")),
            };
            let payload = if self.at(&TokenKind::LParen) {
                self.advance();
                let ty = self.parse_type()?;
                self.expect(&TokenKind::RParen, "`)`")?;
                Some(ty)
            } else {
                None
            };
            variants.push(EnumVariant {
                name: vname,
                payload,
                span: vspan,
            });
            if self.at(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(EnumDecl {
            name,
            variants,
            span,
        })
    }

    fn parse_function(&mut self) -> Result<FunctionDecl, Diagnostic> {
        let span = self.expect(&TokenKind::Function, "`function`")?;
        let name = match self.advance() {
            TokenKind::Identifier(name) => name,
            _ => return Err(self.err_here("Expected function name")),
        };

        self.expect(&TokenKind::LParen, "`(`")?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                let pspan = self.peek_span();
                let pname = match self.advance() {
                    TokenKind::Identifier(n) => n,
                    _ => return Err(self.err_here("Expected parameter name")),
                };
                self.expect(&TokenKind::Colon, "`:`")?;
                let ty = self.parse_type()?;
                params.push(Param {
                    name: pname,
                    ty,
                    span: pspan,
                });
                if self.at(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen, "`)`")?;
        self.expect(&TokenKind::Arrow, "`->`")?;
        let return_type = self.parse_type()?;

        let mut effects = Vec::new();
        if self.at(&TokenKind::Effects) {
            self.advance();
            self.expect(&TokenKind::LParen, "`(`")?;
            if !self.at(&TokenKind::RParen) {
                loop {
                    effects.push(self.parse_effect()?);
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            self.expect(&TokenKind::RParen, "`)`")?;
        }

        self.expect(&TokenKind::LBrace, "`{`")?;
        let mut body = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            body.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace, "`}`")?;

        Ok(FunctionDecl {
            name,
            params,
            return_type,
            effects,
            body,
            span,
        })
    }

    /// Parse an effect path such as `pure`, `database.read`, or
    /// `filesystem.read("/config")`. Argument lists are accepted and ignored.
    fn parse_effect(&mut self) -> Result<String, Diagnostic> {
        let mut name = match self.advance() {
            TokenKind::Identifier(n) => n,
            _ => return Err(self.err_here("Expected effect name")),
        };
        while self.at(&TokenKind::Dot) {
            self.advance();
            match self.advance() {
                TokenKind::Identifier(n) => {
                    name.push('.');
                    name.push_str(&n);
                }
                _ => return Err(self.err_here("Expected effect component after `.`")),
            }
        }
        if self.at(&TokenKind::LParen) {
            let mut depth = 0;
            loop {
                match self.advance() {
                    TokenKind::LParen => depth += 1,
                    TokenKind::RParen => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    TokenKind::Eof => return Err(self.err_here("Unterminated effect arguments")),
                    _ => {}
                }
            }
        }
        Ok(name)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        if self.at(&TokenKind::Let) {
            let span = self.peek_span();
            self.advance();
            let name = match self.advance() {
                TokenKind::Identifier(n) => n,
                _ => return Err(self.err_here("Expected variable name")),
            };
            self.expect(&TokenKind::Assign, "`=`")?;
            let value = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon, "`;`")?;
            Ok(Stmt::Let { name, value, span })
        } else if self.at(&TokenKind::Return) {
            let span = self.peek_span();
            self.advance();
            if self.at(&TokenKind::Semicolon) {
                self.advance();
                Ok(Stmt::Return { value: None, span })
            } else {
                let value = self.parse_expr()?;
                self.expect(&TokenKind::Semicolon, "`;`")?;
                Ok(Stmt::Return {
                    value: Some(value),
                    span,
                })
            }
        } else {
            let expr = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon, "`;`")?;
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_type(&mut self) -> Result<TypeExpr, Diagnostic> {
        let span = self.peek_span();
        let name = match self.advance() {
            TokenKind::Identifier(n) => n,
            _ => return Err(self.err_here("Expected type name")),
        };
        let mut args = Vec::new();
        if self.at(&TokenKind::Lt) {
            self.advance();
            loop {
                args.push(self.parse_type()?);
                if self.at(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(&TokenKind::Gt, "`>`")?;
        }
        Ok(TypeExpr { name, args, span })
    }

    // --- Expressions (precedence climbing) ---

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_and()?;
        while self.at(&TokenKind::OrOr) {
            let span = self.peek_span();
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_equality()?;
        while self.at(&TokenKind::AndAnd) {
            let span = self.peek_span();
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = if self.at(&TokenKind::EqEq) {
                BinOp::Eq
            } else if self.at(&TokenKind::NotEq) {
                BinOp::Ne
            } else {
                break;
            };
            let span = self.peek_span();
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_term()?;
        loop {
            let op = if self.at(&TokenKind::Lt) {
                BinOp::Lt
            } else if self.at(&TokenKind::Le) {
                BinOp::Le
            } else if self.at(&TokenKind::Gt) {
                BinOp::Gt
            } else if self.at(&TokenKind::Ge) {
                BinOp::Ge
            } else {
                break;
            };
            let span = self.peek_span();
            self.advance();
            let right = self.parse_term()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_factor()?;
        loop {
            let op = if self.at(&TokenKind::Plus) {
                BinOp::Add
            } else if self.at(&TokenKind::Minus) {
                BinOp::Sub
            } else {
                break;
            };
            let span = self.peek_span();
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.at(&TokenKind::Star) {
                BinOp::Mul
            } else if self.at(&TokenKind::Slash) {
                BinOp::Div
            } else if self.at(&TokenKind::Percent) {
                BinOp::Rem
            } else {
                break;
            };
            let span = self.peek_span();
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.at(&TokenKind::Minus) {
            let span = self.peek_span();
            self.advance();
            let expr = self.parse_unary()?;
            Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(expr),
                span,
            })
        } else if self.at(&TokenKind::Bang) {
            let span = self.peek_span();
            self.advance();
            let expr = self.parse_unary()?;
            Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
                span,
            })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.at(&TokenKind::LParen) {
                let span = self.peek_span();
                let args = self.parse_args()?;
                expr = match expr {
                    Expr::Ident(name, _) => Expr::Call {
                        callee: name,
                        args,
                        span,
                    },
                    Expr::Member {
                        object,
                        field,
                        span: _,
                    } => Expr::MethodCall {
                        receiver: object,
                        method: field,
                        args,
                        span,
                    },
                    _ => return Err(self.err_here("Only function and method calls are supported")),
                };
            } else if self.at(&TokenKind::Dot) {
                let span = self.peek_span();
                self.advance();
                let field = match self.advance() {
                    TokenKind::Identifier(n) => n,
                    _ => return Err(self.err_here("Expected field name after `.`")),
                };
                expr = Expr::Member {
                    object: Box::new(expr),
                    field,
                    span,
                };
            } else if self.at(&TokenKind::Question) {
                let span = self.peek_span();
                self.advance();
                expr = Expr::Try {
                    expr: Box::new(expr),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        self.expect(&TokenKind::LParen, "`(`")?;
        let mut args = Vec::new();
        if !self.at(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if self.at(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen, "`)`")?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let span = self.peek_span();
        match self.advance() {
            TokenKind::Integer(v) => Ok(Expr::Integer(v, span)),
            TokenKind::Float(v) => Ok(Expr::Float(v, span)),
            TokenKind::StringLit(s) => Ok(Expr::StringLit(s, span)),
            TokenKind::CharLit(c) => Ok(Expr::CharLit(c, span)),
            TokenKind::True => Ok(Expr::Bool(true, span)),
            TokenKind::False => Ok(Expr::Bool(false, span)),
            TokenKind::Identifier(name) => {
                if self.at(&TokenKind::ColonColon) {
                    self.parse_variant(name, span)
                } else if self.at(&TokenKind::LBrace) {
                    self.parse_struct_literal(name, span)
                } else {
                    Ok(Expr::Ident(name, span))
                }
            }
            TokenKind::Pipe => {
                let param = match self.advance() {
                    TokenKind::Identifier(n) => n,
                    _ => return Err(self.err_here("Expected closure parameter name")),
                };
                self.expect(&TokenKind::Pipe, "`|`")?;
                let body = self.parse_expr()?;
                Ok(Expr::Lambda {
                    param,
                    body: Box::new(body),
                    span,
                })
            }
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "`)`")?;
                Ok(Expr::Group(Box::new(expr)))
            }
            other => Err(Diagnostic::error(
                "PARSE_ERROR",
                &self.file,
                span,
                &format!("Unexpected token {:?} in expression", other),
            )),
        }
    }

    fn parse_struct_literal(&mut self, name: String, span: Span) -> Result<Expr, Diagnostic> {
        self.expect(&TokenKind::LBrace, "`{`")?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            let fname = match self.advance() {
                TokenKind::Identifier(n) => n,
                _ => return Err(self.err_here("Expected field name in struct literal")),
            };
            self.expect(&TokenKind::Colon, "`:`")?;
            let value = self.parse_expr()?;
            fields.push((fname, value));
            if self.at(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect(&TokenKind::RBrace, "`}`")?;
        Ok(Expr::StructLit { name, fields, span })
    }

    fn parse_variant(&mut self, enum_name: String, span: Span) -> Result<Expr, Diagnostic> {
        self.expect(&TokenKind::ColonColon, "`::`")?;
        let variant = match self.advance() {
            TokenKind::Identifier(v) => v,
            _ => return Err(self.err_here("Expected variant name after `::`")),
        };
        let payload = if self.at(&TokenKind::LParen) {
            Some(self.parse_args()?)
        } else {
            None
        };
        Ok(Expr::Variant {
            enum_name,
            variant,
            payload,
            span,
        })
    }
}
