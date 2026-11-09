//! Abstract syntax tree for the Phase 1 language subset.

use crate::lexer::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub functions: Vec<FunctionDecl>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub effects: Vec<String>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        value: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    /// `if cond { ... } else { ... }` (also supports `else if`).
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
        span: Span,
    },
    Expr(Expr),
}

impl Stmt {
    /// True when this statement is guaranteed to return from the function.
    ///
    /// Used to decide whether a non-unit function covers every path with an
    /// explicit `return`. An `if` only guarantees a return when it has an
    /// `else` and both branches guarantee one.
    pub fn guarantees_return(&self) -> bool {
        match self {
            Stmt::Return { .. } => true,
            Stmt::If {
                then_body,
                else_body: Some(else_body),
                ..
            } => block_guarantees_return(then_body) && block_guarantees_return(else_body),
            _ => false,
        }
    }
}

/// True when a sequence of statements always returns.
pub fn block_guarantees_return(body: &[Stmt]) -> bool {
    body.iter().any(Stmt::guarantees_return)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i64, Span),
    Float(f64, Span),
    Bool(bool, Span),
    StringLit(String, Span),
    CharLit(char, Span),
    Ident(String, Span),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
        span: Span,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
        span: Span,
    },
    Member {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    /// A path-based enum variant, e.g. `Color::Red` or `Option::Some(x)`.
    Variant {
        enum_name: String,
        variant: String,
        payload: Option<Vec<Expr>>,
        span: Span,
    },
    /// A method call, e.g. `result.unwrap()` or `user.map(|u| u.name)`.
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// The `?` operator.
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    /// A closure such as `|u| u.name`.
    Lambda {
        param: String,
        body: Box<Expr>,
        span: Span,
    },
    /// A `match` expression, e.g. `match opt { Option::Some(x) => x, _ => 0 }`.
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Group(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    /// `_`
    Wildcard { span: Span },
    /// An enum variant pattern, optionally with a payload binding:
    /// `Color::Red`, `Option::Some(x)`, `Some(x)`, `None`.
    Variant {
        /// `Some` in `Option::Some`; `None` when the pattern is unqualified.
        enum_name: Option<String>,
        variant: String,
        /// Whether the pattern wrote `(...)` after the variant name.
        has_parens: bool,
        /// The name bound to the payload, if any (`_` means "ignore").
        binding: Option<String>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard { span } | Pattern::Variant { span, .. } => *span,
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Integer(_, s)
            | Expr::Float(_, s)
            | Expr::Bool(_, s)
            | Expr::StringLit(_, s)
            | Expr::CharLit(_, s)
            | Expr::Ident(_, s) => *s,
            Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Call { span, .. }
            | Expr::Member { span, .. }
            | Expr::StructLit { span, .. }
            | Expr::Variant { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::Try { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Match { span, .. } => *span,
            Expr::Group(e) => e.span(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeExpr {
    pub name: String,
    pub args: Vec<TypeExpr>,
    pub span: Span,
}
