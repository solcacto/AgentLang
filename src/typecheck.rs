//! Static type checker for the Phase 1 language subset.
//!
//! It verifies that every expression has a known type, that operators and
//! calls match, and that functions return what they declare. Failures are
//! reported as structured diagnostics with repair hints.

use std::collections::HashMap;

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::lexer::Span;
use crate::types::Ty;

#[derive(Clone)]
struct FuncSig {
    params: Vec<Ty>,
    ret: Ty,
}

pub struct TypeChecker<'a> {
    file: &'a str,
    functions: HashMap<String, FuncSig>,
    scopes: Vec<HashMap<String, Ty>>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(file: &'a str) -> Self {
        TypeChecker {
            file,
            functions: HashMap::new(),
            scopes: Vec::new(),
        }
    }

    pub fn check_module(&mut self, module: &Module) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        // Register function signatures first so calls can be checked in any order.
        for function in &module.functions {
            if self.functions.contains_key(&function.name) {
                diags.push(
                    Diagnostic::error(
                        "DUPLICATE_SYMBOL",
                        self.file,
                        function.span,
                        &format!("Function `{}` is defined more than once", function.name),
                    )
                    .with_symbol(&function.name),
                );
            }
            let params = function
                .params
                .iter()
                .map(|p| self.resolve_type(&p.ty))
                .collect();
            let ret = self.resolve_type(&function.return_type);
            self.functions
                .insert(function.name.clone(), FuncSig { params, ret });
        }

        for function in &module.functions {
            self.scopes.clear();
            self.scopes.push(HashMap::new());

            for param in &function.params {
                let ty = self.resolve_type(&param.ty);
                if self.scopes.last().unwrap().contains_key(&param.name) {
                    diags.push(
                        Diagnostic::error(
                            "DUPLICATE_SYMBOL",
                            self.file,
                            param.span,
                            &format!("Parameter `{}` is declared more than once", param.name),
                        )
                        .with_symbol(&param.name),
                    );
                }
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(param.name.clone(), ty);
            }

            let ret_ty = self.resolve_type(&function.return_type);
            let mut saw_return = false;
            for stmt in &function.body {
                self.check_stmt(stmt, &ret_ty, &mut diags, &mut saw_return);
            }

            if ret_ty != Ty::Unit && ret_ty != Ty::Unknown && !saw_return {
                diags.push(
                    Diagnostic::error(
                        "MISSING_RETURN",
                        self.file,
                        function.span,
                        &format!(
                            "Function `{}` must return a value of type `{}`",
                            function.name, ret_ty
                        ),
                    )
                    .with_symbol(&function.name)
                    .with_repair("insert_return", Some(&function.name), Some(&ret_ty.to_string())),
                );
            }
        }

        diags
    }

    fn resolve_type(&self, ty: &TypeExpr) -> Ty {
        if let Some(builtin) = Ty::from_name(&ty.name) {
            return builtin;
        }
        match ty.name.as_str() {
            "Array" => Ty::Array(Box::new(self.resolve_arg(ty, 0))),
            "Option" => Ty::Option(Box::new(self.resolve_arg(ty, 0))),
            "Result" => Ty::Result(
                Box::new(self.resolve_arg(ty, 0)),
                Box::new(self.resolve_arg(ty, 1)),
            ),
            "Map" => Ty::Map(
                Box::new(self.resolve_arg(ty, 0)),
                Box::new(self.resolve_arg(ty, 1)),
            ),
            other => Ty::Named(other.to_string()),
        }
    }

    fn resolve_arg(&self, ty: &TypeExpr, index: usize) -> Ty {
        ty.args
            .get(index)
            .map(|arg| self.resolve_type(arg))
            .unwrap_or(Ty::Unknown)
    }

    fn lookup(&self, name: &str) -> Option<Ty> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        ret_ty: &Ty,
        diags: &mut Vec<Diagnostic>,
        saw_return: &mut bool,
    ) {
        match stmt {
            Stmt::Let { name, value, span } => {
                let ty = self.check_expr(value, diags);
                if self.scopes.last().unwrap().contains_key(name) {
                    diags.push(
                        Diagnostic::error(
                            "DUPLICATE_SYMBOL",
                            self.file,
                            *span,
                            &format!("Variable `{}` is declared more than once in this scope", name),
                        )
                        .with_symbol(name),
                    );
                }
                self.scopes.last_mut().unwrap().insert(name.clone(), ty);
            }
            Stmt::Return { value, span } => {
                *saw_return = true;
                let actual = match value {
                    Some(expr) => self.check_expr(expr, diags),
                    None => Ty::Unit,
                };
                if actual != Ty::Unknown && *ret_ty != Ty::Unknown && actual != *ret_ty {
                    diags.push(
                        Diagnostic::error(
                            "TYPE_MISMATCH",
                            self.file,
                            *span,
                            &format!(
                                "Return type mismatch: expected `{}`, found `{}`",
                                ret_ty, actual
                            ),
                        )
                        .with_types(&ret_ty.to_string(), &actual.to_string())
                        .with_repair("replace_expression", None, Some(&ret_ty.to_string())),
                    );
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, diags);
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr, diags: &mut Vec<Diagnostic>) -> Ty {
        match expr {
            Expr::Integer(_, _) => Ty::I32,
            Expr::Float(_, _) => Ty::F64,
            Expr::Bool(_, _) => Ty::Bool,
            Expr::StringLit(_, _) => Ty::String,
            Expr::CharLit(_, _) => Ty::Char,
            Expr::Group(inner) => self.check_expr(inner, diags),
            Expr::Ident(name, span) => {
                if let Some(ty) = self.lookup(name) {
                    ty
                } else if self.functions.contains_key(name) {
                    // Using a function name as a value is not supported yet.
                    Ty::Unknown
                } else {
                    diags.push(
                        Diagnostic::error(
                            "UNDEFINED_SYMBOL",
                            self.file,
                            *span,
                            &format!("Undefined symbol `{}`", name),
                        )
                        .with_symbol(name)
                        .with_repair("declare_symbol", Some(name), None),
                    );
                    Ty::Unknown
                }
            }
            Expr::Unary { op, expr, span } => {
                let ty = self.check_expr(expr, diags);
                match op {
                    UnOp::Neg => {
                        if ty != Ty::Unknown && !ty.is_numeric() {
                            diags.push(
                                Diagnostic::error(
                                    "TYPE_MISMATCH",
                                    self.file,
                                    *span,
                                    &format!(
                                        "Unary `-` requires a numeric operand, found `{}`",
                                        ty
                                    ),
                                )
                                .with_types("numeric", &ty.to_string()),
                            );
                        }
                        ty
                    }
                    UnOp::Not => {
                        if ty != Ty::Unknown && ty != Ty::Bool {
                            diags.push(
                                Diagnostic::error(
                                    "TYPE_MISMATCH",
                                    self.file,
                                    *span,
                                    &format!("Unary `!` requires `Bool`, found `{}`", ty),
                                )
                                .with_types("Bool", &ty.to_string()),
                            );
                        }
                        Ty::Bool
                    }
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let left_ty = self.check_expr(left, diags);
                let right_ty = self.check_expr(right, diags);
                self.check_binary(*op, &left_ty, &right_ty, *span, diags)
            }
            Expr::Call {
                callee,
                args,
                span,
            } => self.check_call(callee, args, *span, diags),
            Expr::Member {
                object,
                field: _,
                span: _,
            } => {
                // Struct/enum field resolution lands in Milestone 4.
                let _ = self.check_expr(object, diags);
                Ty::Unknown
            }
        }
    }

    fn check_call(
        &mut self,
        callee: &str,
        args: &[Expr],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        let mut arg_types = Vec::new();
        for arg in args {
            arg_types.push(self.check_expr(arg, diags));
        }

        let sig = match self.functions.get(callee).cloned() {
            Some(sig) => sig,
            None => {
                diags.push(
                    Diagnostic::error(
                        "UNDEFINED_FUNCTION",
                        self.file,
                        span,
                        &format!("Undefined function `{}`", callee),
                    )
                    .with_symbol(callee)
                    .with_repair("add_import", Some(callee), None),
                );
                return Ty::Unknown;
            }
        };

        if arg_types.len() != sig.params.len() {
            diags.push(
                Diagnostic::error(
                    "ARITY_MISMATCH",
                    self.file,
                    span,
                    &format!(
                        "Function `{}` expects {} argument(s), found {}",
                        callee,
                        sig.params.len(),
                        arg_types.len()
                    ),
                )
                .with_symbol(callee)
                .with_types(
                    &format!("{} argument(s)", sig.params.len()),
                    &format!("{}", arg_types.len()),
                ),
            );
            return sig.ret;
        }

        for (index, (actual, expected)) in arg_types.iter().zip(sig.params.iter()).enumerate() {
            if *actual != Ty::Unknown && *expected != Ty::Unknown && actual != expected {
                diags.push(
                    Diagnostic::error(
                        "TYPE_MISMATCH",
                        self.file,
                        span,
                        &format!(
                            "Argument {} to `{}`: expected `{}`, found `{}`",
                            index + 1,
                            callee,
                            expected,
                            actual
                        ),
                    )
                    .with_symbol(callee)
                    .with_types(&expected.to_string(), &actual.to_string())
                    .with_repair("replace_expression", None, Some(&expected.to_string())),
                );
            }
        }

        sig.ret
    }

    fn check_binary(
        &self,
        op: BinOp,
        left: &Ty,
        right: &Ty,
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        use BinOp::*;
        match op {
            Add | Sub | Mul | Div | Rem => {
                if *left == Ty::Unknown || *right == Ty::Unknown {
                    return Ty::Unknown;
                }
                if !left.is_numeric() || !right.is_numeric() || left != right {
                    diags.push(
                        Diagnostic::error(
                            "TYPE_MISMATCH",
                            self.file,
                            span,
                            &format!(
                                "Operator `{:?}` requires matching numeric operands, found `{}` and `{}`",
                                op, left, right
                            ),
                        )
                        .with_types(&left.to_string(), &right.to_string())
                        .with_repair("replace_expression", None, Some(&left.to_string())),
                    );
                    return Ty::Unknown;
                }
                left.clone()
            }
            Eq | Ne => Ty::Bool,
            Lt | Le | Gt | Ge => {
                if *left != Ty::Unknown
                    && *right != Ty::Unknown
                    && !(left.is_numeric() && right.is_numeric())
                {
                    diags.push(
                        Diagnostic::error(
                            "TYPE_MISMATCH",
                            self.file,
                            span,
                            &format!(
                                "Comparison requires numeric operands, found `{}` and `{}`",
                                left, right
                            ),
                        )
                        .with_types(&left.to_string(), &right.to_string()),
                    );
                }
                Ty::Bool
            }
            And | Or => {
                if *left != Ty::Unknown && *left != Ty::Bool {
                    diags.push(
                        Diagnostic::error(
                            "TYPE_MISMATCH",
                            self.file,
                            span,
                            &format!("Logical operator requires `Bool`, found `{}`", left),
                        )
                        .with_types("Bool", &left.to_string()),
                    );
                }
                if *right != Ty::Unknown && *right != Ty::Bool {
                    diags.push(
                        Diagnostic::error(
                            "TYPE_MISMATCH",
                            self.file,
                            span,
                            &format!("Logical operator requires `Bool`, found `{}`", right),
                        )
                        .with_types("Bool", &right.to_string()),
                    );
                }
                Ty::Bool
            }
        }
    }
}
