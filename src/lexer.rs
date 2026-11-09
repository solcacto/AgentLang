//! Lexer: turns source text into a token stream with precise source locations.

use crate::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self {
        Span { line, col }
    }

    pub fn zero() -> Self {
        Span { line: 1, col: 1 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Function,
    Return,
    Let,
    Struct,
    Enum,
    Type,
    Requires,
    Ensures,
    Effects,
    Where,
    If,
    Else,
    Match,
    While,
    For,
    In,
    Mut,
    Parallel,
    True,
    False,
    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    ColonColon,
    Semicolon,
    Dot,
    Arrow,
    FatArrow,
    Pipe,
    Question,
    Assign,
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    // Literals
    Identifier(String),
    Integer(i64),
    Float(f64),
    StringLit(String),
    CharLit(char),
    // End of input
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    file: String,
}

impl Lexer {
    pub fn new(source: &str, file: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            file: file.to_string(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.col)
    }

    fn err(&self, span: Span, msg: &str) -> Diagnostic {
        Diagnostic::error("LEX_ERROR", &self.file, span, msg)
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments()?;
            let span = self.span();
            let c = match self.peek() {
                Some(c) => c,
                None => {
                    tokens.push(Token {
                        kind: TokenKind::Eof,
                        span,
                    });
                    break;
                }
            };

            let kind = match c {
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                '{' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    TokenKind::RBrace
                }
                '[' => {
                    self.advance();
                    TokenKind::LBracket
                }
                ']' => {
                    self.advance();
                    TokenKind::RBracket
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                ':' => {
                    self.advance();
                    if self.peek() == Some(':') {
                        self.advance();
                        TokenKind::ColonColon
                    } else {
                        TokenKind::Colon
                    }
                }
                ';' => {
                    self.advance();
                    TokenKind::Semicolon
                }
                '.' => {
                    self.advance();
                    TokenKind::Dot
                }
                '?' => {
                    self.advance();
                    TokenKind::Question
                }
                '|' => {
                    self.advance();
                    if self.peek() == Some('|') {
                        self.advance();
                        TokenKind::OrOr
                    } else {
                        TokenKind::Pipe
                    }
                }
                '&' => {
                    self.advance();
                    if self.peek() == Some('&') {
                        self.advance();
                        TokenKind::AndAnd
                    } else {
                        return Err(self.err(span, "Unexpected character '&'"));
                    }
                }
                '+' => {
                    self.advance();
                    TokenKind::Plus
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '/' => {
                    self.advance();
                    TokenKind::Slash
                }
                '%' => {
                    self.advance();
                    TokenKind::Percent
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::EqEq
                    } else if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::FatArrow
                    } else {
                        TokenKind::Assign
                    }
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::Le
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::Ge
                    } else {
                        TokenKind::Gt
                    }
                }
                '-' => {
                    self.advance();
                    if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    }
                }
                '"' => self.lex_string(span)?,
                '\'' => self.lex_char(span)?,
                c if c.is_ascii_digit() => self.lex_number(span)?,
                c if is_ident_start(c) => self.lex_ident(),
                other => return Err(self.err(span, &format!("Unexpected character '{}'", other))),
            };

            tokens.push(Token { kind, span });
        }
        Ok(tokens)
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), Diagnostic> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let start = self.span();
                    self.advance();
                    self.advance();
                    loop {
                        match self.peek() {
                            Some('*') if self.peek_at(1) == Some('/') => {
                                self.advance();
                                self.advance();
                                break;
                            }
                            Some(_) => {
                                self.advance();
                            }
                            None => return Err(self.err(start, "Unterminated block comment")),
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn lex_string(&mut self, span: Span) -> Result<TokenKind, Diagnostic> {
        self.advance(); // opening quote
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some('0') => s.push('\0'),
                    Some(c) => {
                        s.push('\\');
                        s.push(c);
                    }
                    None => return Err(self.err(span, "Unterminated string literal")),
                },
                Some(c) => s.push(c),
                None => return Err(self.err(span, "Unterminated string literal")),
            }
        }
        Ok(TokenKind::StringLit(s))
    }

    fn lex_char(&mut self, span: Span) -> Result<TokenKind, Diagnostic> {
        self.advance(); // opening quote
        let c = match self.advance() {
            Some('\\') => match self.advance() {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('r') => '\r',
                Some('\\') => '\\',
                Some('\'') => '\'',
                Some(c) => c,
                None => return Err(self.err(span, "Unterminated char literal")),
            },
            Some(c) => c,
            None => return Err(self.err(span, "Unterminated char literal")),
        };
        match self.advance() {
            Some('\'') => Ok(TokenKind::CharLit(c)),
            _ => Err(self.err(span, "Unterminated char literal")),
        }
    }

    fn lex_number(&mut self, span: Span) -> Result<TokenKind, Diagnostic> {
        let mut num = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let mut is_float = false;
        if self.peek() == Some('.') && self.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
            is_float = true;
            num.push('.');
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    num.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if is_float {
            num.parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| self.err(span, "Invalid float literal"))
        } else {
            num.parse::<i64>()
                .map(TokenKind::Integer)
                .map_err(|_| self.err(span, "Invalid integer literal"))
        }
    }

    fn lex_ident(&mut self) -> TokenKind {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match s.as_str() {
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "let" => TokenKind::Let,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "type" => TokenKind::Type,
            "requires" => TokenKind::Requires,
            "ensures" => TokenKind::Ensures,
            "effects" => TokenKind::Effects,
            "where" => TokenKind::Where,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "mut" => TokenKind::Mut,
            "parallel" => TokenKind::Parallel,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Identifier(s),
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
