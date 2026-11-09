//! Static type checker for the Phase 1 language subset.
//!
//! It verifies that every expression has a known type, that operators and
//! calls match, and that functions return what they declare. Failures are
//! reported as structured diagnostics with repair hints.

use std::collections::{HashMap, HashSet};

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
    structs: HashMap<String, HashMap<String, Ty>>,
    /// Enum name -> (variant name -> optional payload type).
    enums: HashMap<String, HashMap<String, Option<Ty>>>,
    scopes: Vec<HashMap<String, Ty>>,
    /// Return type of the function currently being checked (for `?`).
    current_return: Ty,
}

impl<'a> TypeChecker<'a> {
    pub fn new(file: &'a str) -> Self {
        TypeChecker {
            file,
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            scopes: Vec::new(),
            current_return: Ty::Unknown,
        }
    }

    pub fn check_module(&mut self, module: &Module) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        // Register struct definitions first.
        for decl in &module.structs {
            if self.structs.contains_key(&decl.name) || self.enums.contains_key(&decl.name) {
                diags.push(
                    Diagnostic::error(
                        "DUPLICATE_SYMBOL",
                        self.file,
                        decl.span,
                        &format!("Type `{}` is defined more than once", decl.name),
                    )
                    .with_symbol(&decl.name),
                );
            }
            let mut fields = HashMap::new();
            for field in &decl.fields {
                let ty = self.resolve_type(&field.ty);
                if fields.contains_key(&field.name) {
                    diags.push(
                        Diagnostic::error(
                            "DUPLICATE_FIELD",
                            self.file,
                            field.span,
                            &format!(
                                "Field `{}` is declared more than once in `{}`",
                                field.name, decl.name
                            ),
                        )
                        .with_symbol(&field.name),
                    );
                }
                fields.insert(field.name.clone(), ty);
            }
            self.structs.insert(decl.name.clone(), fields);
        }

        // Register enum definitions.
        for decl in &module.enums {
            if self.structs.contains_key(&decl.name) || self.enums.contains_key(&decl.name) {
                diags.push(
                    Diagnostic::error(
                        "DUPLICATE_SYMBOL",
                        self.file,
                        decl.span,
                        &format!("Type `{}` is defined more than once", decl.name),
                    )
                    .with_symbol(&decl.name),
                );
            }
            let mut variants = HashMap::new();
            for variant in &decl.variants {
                let payload = variant.payload.as_ref().map(|ty| self.resolve_type(ty));
                if variants.contains_key(&variant.name) {
                    diags.push(
                        Diagnostic::error(
                            "DUPLICATE_SYMBOL",
                            self.file,
                            variant.span,
                            &format!(
                                "Variant `{}` is declared more than once in `{}`",
                                variant.name, decl.name
                            ),
                        )
                        .with_symbol(&variant.name),
                    );
                }
                variants.insert(variant.name.clone(), payload);
            }
            self.enums.insert(decl.name.clone(), variants);
        }

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
            self.current_return = ret_ty.clone();
            for stmt in &function.body {
                self.check_stmt(stmt, &ret_ty, &mut diags);
            }

            if ret_ty != Ty::Unit
                && ret_ty != Ty::Unknown
                && !block_guarantees_return(&function.body)
            {
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
                    .with_repair(
                        "insert_return",
                        Some(&function.name),
                        Some(&ret_ty.to_string()),
                    ),
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

    fn check_stmt(&mut self, stmt: &Stmt, ret_ty: &Ty, diags: &mut Vec<Diagnostic>) {
        match stmt {
            Stmt::Let { name, value, span } => {
                let ty = self.check_expr(value, diags);
                if self.scopes.last().unwrap().contains_key(name) {
                    diags.push(
                        Diagnostic::error(
                            "DUPLICATE_SYMBOL",
                            self.file,
                            *span,
                            &format!(
                                "Variable `{}` is declared more than once in this scope",
                                name
                            ),
                        )
                        .with_symbol(name),
                    );
                }
                self.scopes.last_mut().unwrap().insert(name.clone(), ty);
            }
            Stmt::Return { value, span } => {
                let actual = match value {
                    Some(expr) => self.check_expr(expr, diags),
                    None => Ty::Unit,
                };
                if !compatible(ret_ty, &actual) {
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
                        .with_repair(
                            "replace_expression",
                            None,
                            Some(&ret_ty.to_string()),
                        ),
                    );
                }
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                span: _,
            } => {
                let cond_ty = self.check_expr(cond, diags);
                if !compatible(&Ty::Bool, &cond_ty) {
                    diags.push(
                        Diagnostic::error(
                            "TYPE_MISMATCH",
                            self.file,
                            cond.span(),
                            &format!("`if` condition must be `Bool`, found `{}`", cond_ty),
                        )
                        .with_types("Bool", &cond_ty.to_string())
                        .with_repair(
                            "replace_expression",
                            None,
                            Some("Bool"),
                        ),
                    );
                }
                self.check_block(then_body, ret_ty, diags);
                if let Some(else_body) = else_body {
                    self.check_block(else_body, ret_ty, diags);
                }
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, diags);
            }
        }
    }

    /// Check a block in a fresh scope so declarations do not leak out.
    fn check_block(&mut self, body: &[Stmt], ret_ty: &Ty, diags: &mut Vec<Diagnostic>) {
        self.scopes.push(HashMap::new());
        for stmt in body {
            self.check_stmt(stmt, ret_ty, diags);
        }
        self.scopes.pop();
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
                } else if name == "None" {
                    Ty::Option(Box::new(Ty::Unknown))
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
            Expr::Call { callee, args, span } => self.check_call(callee, args, *span, diags),
            Expr::Member {
                object,
                field,
                span,
            } => {
                let object_ty = self.check_expr(object, diags);
                match object_ty {
                    Ty::Named(struct_name) => {
                        if let Some(fields) = self.structs.get(&struct_name) {
                            match fields.get(field) {
                                Some(field_ty) => field_ty.clone(),
                                None => {
                                    diags.push(
                                        Diagnostic::error(
                                            "UNKNOWN_FIELD",
                                            self.file,
                                            *span,
                                            &format!(
                                                "Struct `{}` has no field `{}`",
                                                struct_name, field
                                            ),
                                        )
                                        .with_symbol(field)
                                        .with_repair(
                                            "rename_symbol",
                                            Some(field),
                                            None,
                                        ),
                                    );
                                    Ty::Unknown
                                }
                            }
                        } else {
                            // Enum or unknown named type: member access unsupported.
                            Ty::Unknown
                        }
                    }
                    Ty::Unknown => Ty::Unknown,
                    other => {
                        diags.push(
                            Diagnostic::error(
                                "NOT_A_STRUCT",
                                self.file,
                                *span,
                                &format!(
                                    "Cannot access field `{}` on non-struct type `{}`",
                                    field, other
                                ),
                            )
                            .with_types("<struct>", &other.to_string()),
                        );
                        Ty::Unknown
                    }
                }
            }
            Expr::StructLit { name, fields, span } => {
                self.check_struct_literal(name, fields, *span, diags)
            }
            Expr::Variant {
                enum_name,
                variant,
                payload,
                span,
            } => self.check_variant(enum_name, variant, payload.as_deref(), *span, diags),
            Expr::MethodCall {
                receiver,
                method,
                args,
                span,
            } => self.check_method_call(receiver, method, args, *span, diags),
            Expr::Try { expr, span } => self.check_try(expr, *span, diags),
            Expr::Lambda { .. } => {
                diags.push(Diagnostic::error(
                    "UNSUPPORTED_LAMBDA",
                    self.file,
                    expr.span(),
                    "Closures are only supported as arguments to `.map` in Phase 1",
                ));
                Ty::Unknown
            }
            Expr::Match {
                scrutinee,
                arms,
                span,
            } => self.check_match(scrutinee, arms, *span, diags),
        }
    }

    fn enum_variants_of(&self, ty: &Ty) -> Option<HashMap<String, Option<Ty>>> {
        match ty {
            Ty::Named(name) => self.enums.get(name).cloned(),
            Ty::Option(inner) => {
                let mut variants = HashMap::new();
                variants.insert("Some".to_string(), Some((**inner).clone()));
                variants.insert("None".to_string(), None);
                Some(variants)
            }
            Ty::Result(ok, err) => {
                let mut variants = HashMap::new();
                variants.insert("Ok".to_string(), Some((**ok).clone()));
                variants.insert("Err".to_string(), Some((**err).clone()));
                Some(variants)
            }
            _ => None,
        }
    }

    fn check_match(
        &mut self,
        scrutinee: &Expr,
        arms: &[MatchArm],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        let scrutinee_ty = self.check_expr(scrutinee, diags);
        let variants = self.enum_variants_of(&scrutinee_ty);

        // Variant patterns only make sense on an enum. Report once, rather than
        // per arm, when the scrutinee is a known non-enum type.
        let has_variant_pattern = arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Variant { .. }));
        if variants.is_none() && has_variant_pattern && !matches!(scrutinee_ty, Ty::Unknown) {
            diags.push(
                Diagnostic::error(
                    "NOT_AN_ENUM",
                    self.file,
                    span,
                    &format!(
                        "`match` variant patterns require an enum, found `{}`",
                        scrutinee_ty
                    ),
                )
                .with_types("<enum>", &scrutinee_ty.to_string()),
            );
        }

        let mut covered: HashSet<String> = HashSet::new();
        let mut saw_wildcard = false;
        let mut result_ty: Option<Ty> = None;

        for arm in arms {
            self.scopes.push(HashMap::new());

            if let Some(variants) = &variants {
                self.check_pattern(
                    &arm.pattern,
                    variants,
                    &scrutinee_ty,
                    &mut covered,
                    &mut saw_wildcard,
                    diags,
                );
            } else if matches!(arm.pattern, Pattern::Wildcard { .. }) {
                saw_wildcard = true;
            }

            let body_ty = self.check_expr(&arm.body, diags);
            self.scopes.pop();

            match &result_ty {
                None => result_ty = Some(body_ty),
                Some(expected) => {
                    if !compatible(expected, &body_ty) {
                        diags.push(
                            Diagnostic::error(
                                "TYPE_MISMATCH",
                                self.file,
                                arm.body.span(),
                                &format!(
                                    "`match` arm type mismatch: expected `{}`, found `{}`",
                                    expected, body_ty
                                ),
                            )
                            .with_types(&expected.to_string(), &body_ty.to_string())
                            .with_repair(
                                "replace_expression",
                                None,
                                Some(&expected.to_string()),
                            ),
                        );
                    }
                }
            }
        }

        if let Some(variants) = &variants {
            if !saw_wildcard {
                let mut missing: Vec<&String> = variants
                    .keys()
                    .filter(|name| !covered.contains(*name))
                    .collect();
                missing.sort();
                for name in missing {
                    diags.push(
                        Diagnostic::error(
                            "NON_EXHAUSTIVE_MATCH",
                            self.file,
                            span,
                            &format!("Non-exhaustive `match`: no arm covers variant `{}`", name),
                        )
                        .with_symbol(name)
                        .with_repair("add_match_arm", Some(name), None),
                    );
                }
            }
        }

        result_ty.unwrap_or(Ty::Unknown)
    }

    fn check_pattern(
        &mut self,
        pattern: &Pattern,
        variants: &HashMap<String, Option<Ty>>,
        scrutinee_ty: &Ty,
        covered: &mut HashSet<String>,
        saw_wildcard: &mut bool,
        diags: &mut Vec<Diagnostic>,
    ) {
        let Pattern::Variant {
            enum_name,
            variant,
            has_parens,
            binding,
            span,
        } = pattern
        else {
            *saw_wildcard = true;
            return;
        };

        if let Some(qualified) = enum_name {
            let actual = match scrutinee_ty {
                Ty::Named(name) => name.as_str(),
                Ty::Option(_) => "Option",
                Ty::Result(_, _) => "Result",
                _ => "",
            };
            if qualified != actual {
                diags.push(
                    Diagnostic::error(
                        "TYPE_MISMATCH",
                        self.file,
                        *span,
                        &format!(
                            "Pattern `{}::{}` does not match scrutinee type `{}`",
                            qualified, variant, scrutinee_ty
                        ),
                    )
                    .with_symbol(variant)
                    .with_types(actual, qualified),
                );
            }
        }

        match variants.get(variant) {
            None => {
                diags.push(
                    Diagnostic::error(
                        "UNKNOWN_VARIANT",
                        self.file,
                        *span,
                        &format!("`{}` has no variant `{}`", scrutinee_ty, variant),
                    )
                    .with_symbol(variant)
                    .with_repair("rename_symbol", Some(variant), None),
                );
            }
            Some(None) => {
                if *has_parens {
                    diags.push(
                        Diagnostic::error(
                            "ARITY_MISMATCH",
                            self.file,
                            *span,
                            &format!("Variant `{}` takes no payload", variant),
                        )
                        .with_types("0 payload(s)", "1 payload"),
                    );
                }
            }
            Some(Some(payload_ty)) => match binding {
                Some(name) if *has_parens => {
                    if name != "_" {
                        self.scopes
                            .last_mut()
                            .unwrap()
                            .insert(name.clone(), payload_ty.clone());
                    }
                }
                _ => {
                    diags.push(
                        Diagnostic::error(
                            "ARITY_MISMATCH",
                            self.file,
                            *span,
                            &format!(
                                "Variant `{}` has a `{}` payload; bind it as `{}(name)`",
                                variant, payload_ty, variant
                            ),
                        )
                        .with_types("1 payload(s)", "0 payloads"),
                    );
                }
            },
        }

        if !covered.insert(variant.clone()) {
            diags.push(
                Diagnostic::error(
                    "DUPLICATE_MATCH_ARM",
                    self.file,
                    *span,
                    &format!("Variant `{}` is matched more than once", variant),
                )
                .with_symbol(variant),
            );
        }
    }

    fn check_struct_literal(
        &mut self,
        name: &str,
        fields: &[(String, Expr)],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        let struct_fields = match self.structs.get(name).cloned() {
            Some(fields) => fields,
            None => {
                diags.push(
                    Diagnostic::error(
                        "UNKNOWN_TYPE",
                        self.file,
                        span,
                        &format!("Unknown struct `{}`", name),
                    )
                    .with_symbol(name)
                    .with_repair("declare_symbol", Some(name), None),
                );
                for (_, value) in fields {
                    self.check_expr(value, diags);
                }
                return Ty::Unknown;
            }
        };

        let mut seen: HashSet<String> = HashSet::new();
        for (field_name, value) in fields {
            let value_ty = self.check_expr(value, diags);
            if !seen.insert(field_name.clone()) {
                diags.push(
                    Diagnostic::error(
                        "DUPLICATE_FIELD",
                        self.file,
                        value.span(),
                        &format!("Field `{}` initialized more than once", field_name),
                    )
                    .with_symbol(field_name),
                );
            }
            match struct_fields.get(field_name) {
                Some(expected) => {
                    if !compatible(expected, &value_ty) {
                        diags.push(
                            Diagnostic::error(
                                "TYPE_MISMATCH",
                                self.file,
                                value.span(),
                                &format!(
                                    "Field `{}` of `{}`: expected `{}`, found `{}`",
                                    field_name, name, expected, value_ty
                                ),
                            )
                            .with_symbol(field_name)
                            .with_types(&expected.to_string(), &value_ty.to_string())
                            .with_repair(
                                "replace_expression",
                                None,
                                Some(&expected.to_string()),
                            ),
                        );
                    }
                }
                None => {
                    diags.push(
                        Diagnostic::error(
                            "UNKNOWN_FIELD",
                            self.file,
                            value.span(),
                            &format!("Struct `{}` has no field `{}`", name, field_name),
                        )
                        .with_symbol(field_name)
                        .with_repair(
                            "rename_symbol",
                            Some(field_name),
                            None,
                        ),
                    );
                }
            }
        }

        for (field_name, field_ty) in &struct_fields {
            if !seen.contains(field_name) {
                diags.push(
                    Diagnostic::error(
                        "MISSING_FIELD",
                        self.file,
                        span,
                        &format!("Missing field `{}` in `{}` literal", field_name, name),
                    )
                    .with_symbol(field_name)
                    .with_repair(
                        "add_field",
                        Some(field_name),
                        Some(&field_ty.to_string()),
                    ),
                );
            }
        }

        Ty::Named(name.to_string())
    }

    fn check_call(
        &mut self,
        callee: &str,
        args: &[Expr],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        // Unqualified Option/Result constructors (`Some`, `Ok`, `Err`) are
        // available unless the user has defined a function with that name.
        if !self.functions.contains_key(callee) {
            if let Some(ty) = self.check_builtin_constructor(callee, args, span, diags) {
                return ty;
            }
        }

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
            if !compatible(expected, actual) {
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
                    .with_repair(
                        "replace_expression",
                        None,
                        Some(&expected.to_string()),
                    ),
                );
            }
        }

        sig.ret
    }

    /// Check an unqualified `Some` / `None` / `Ok` / `Err` constructor.
    /// Returns `None` when `name` is not a built-in constructor.
    fn check_builtin_constructor(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<Ty> {
        match name {
            "Some" => Some(self.expect_one_payload("Some", args, span, diags, |inner| {
                Ty::Option(Box::new(inner))
            })),
            "Ok" => Some(self.expect_one_payload("Ok", args, span, diags, |inner| {
                Ty::Result(Box::new(inner), Box::new(Ty::Unknown))
            })),
            "Err" => Some(self.expect_one_payload("Err", args, span, diags, |inner| {
                Ty::Result(Box::new(Ty::Unknown), Box::new(inner))
            })),
            "None" => {
                if !args.is_empty() {
                    diags.push(
                        Diagnostic::error(
                            "ARITY_MISMATCH",
                            self.file,
                            span,
                            "`None` takes no arguments",
                        )
                        .with_types("0 argument(s)", &format!("{}", args.len())),
                    );
                    for arg in args {
                        self.check_expr(arg, diags);
                    }
                }
                Some(Ty::Option(Box::new(Ty::Unknown)))
            }
            _ => None,
        }
    }

    fn expect_one_payload(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
        diags: &mut Vec<Diagnostic>,
        wrap: impl FnOnce(Ty) -> Ty,
    ) -> Ty {
        if args.len() != 1 {
            diags.push(
                Diagnostic::error(
                    "ARITY_MISMATCH",
                    self.file,
                    span,
                    &format!("`{}` expects exactly one argument", name),
                )
                .with_types("1 argument(s)", &format!("{}", args.len())),
            );
            for arg in args {
                self.check_expr(arg, diags);
            }
            return Ty::Unknown;
        }
        let inner = self.check_expr(&args[0], diags);
        wrap(inner)
    }

    fn check_variant(
        &mut self,
        enum_name: &str,
        variant: &str,
        payload: Option<&[Expr]>,
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        let args = payload.unwrap_or(&[]);
        match enum_name {
            "Option" => match variant {
                "Some" => self.expect_one_payload("Option::Some", args, span, diags, |inner| {
                    Ty::Option(Box::new(inner))
                }),
                "None" => {
                    self.reject_payload("Option::None", args, span, diags);
                    Ty::Option(Box::new(Ty::Unknown))
                }
                _ => {
                    diags.push(
                        Diagnostic::error(
                            "UNKNOWN_VARIANT",
                            self.file,
                            span,
                            &format!("`Option` has no variant `{}`", variant),
                        )
                        .with_symbol(variant)
                        .with_repair("rename_symbol", Some(variant), None),
                    );
                    Ty::Unknown
                }
            },
            "Result" => match variant {
                "Ok" => self.expect_one_payload("Result::Ok", args, span, diags, |inner| {
                    Ty::Result(Box::new(inner), Box::new(Ty::Unknown))
                }),
                "Err" => self.expect_one_payload("Result::Err", args, span, diags, |inner| {
                    Ty::Result(Box::new(Ty::Unknown), Box::new(inner))
                }),
                _ => {
                    diags.push(
                        Diagnostic::error(
                            "UNKNOWN_VARIANT",
                            self.file,
                            span,
                            &format!("`Result` has no variant `{}`", variant),
                        )
                        .with_symbol(variant)
                        .with_repair("rename_symbol", Some(variant), None),
                    );
                    Ty::Unknown
                }
            },
            _ => {
                let variants = match self.enums.get(enum_name).cloned() {
                    Some(variants) => variants,
                    None => {
                        diags.push(
                            Diagnostic::error(
                                "UNKNOWN_TYPE",
                                self.file,
                                span,
                                &format!("Unknown enum `{}`", enum_name),
                            )
                            .with_symbol(enum_name)
                            .with_repair(
                                "declare_symbol",
                                Some(enum_name),
                                None,
                            ),
                        );
                        for arg in args {
                            self.check_expr(arg, diags);
                        }
                        return Ty::Unknown;
                    }
                };

                match variants.get(variant) {
                    Some(Some(expected)) => {
                        if args.len() != 1 {
                            diags.push(
                                Diagnostic::error(
                                    "ARITY_MISMATCH",
                                    self.file,
                                    span,
                                    &format!(
                                        "Variant `{}::{}` expects one payload",
                                        enum_name, variant
                                    ),
                                )
                                .with_types("1 argument(s)", &format!("{}", args.len())),
                            );
                            for arg in args {
                                self.check_expr(arg, diags);
                            }
                        } else {
                            let actual = self.check_expr(&args[0], diags);
                            if !compatible(expected, &actual) {
                                diags.push(
                                    Diagnostic::error(
                                        "TYPE_MISMATCH",
                                        self.file,
                                        span,
                                        &format!(
                                            "Payload for `{}::{}`: expected `{}`, found `{}`",
                                            enum_name, variant, expected, actual
                                        ),
                                    )
                                    .with_symbol(variant)
                                    .with_types(&expected.to_string(), &actual.to_string())
                                    .with_repair(
                                        "replace_expression",
                                        None,
                                        Some(&expected.to_string()),
                                    ),
                                );
                            }
                        }
                    }
                    Some(None) => {
                        self.reject_payload(
                            &format!("{}::{}", enum_name, variant),
                            args,
                            span,
                            diags,
                        );
                    }
                    None => {
                        diags.push(
                            Diagnostic::error(
                                "UNKNOWN_VARIANT",
                                self.file,
                                span,
                                &format!("Enum `{}` has no variant `{}`", enum_name, variant),
                            )
                            .with_symbol(variant)
                            .with_repair(
                                "rename_symbol",
                                Some(variant),
                                None,
                            ),
                        );
                        for arg in args {
                            self.check_expr(arg, diags);
                        }
                    }
                }

                Ty::Named(enum_name.to_string())
            }
        }
    }

    fn reject_payload(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) {
        if !args.is_empty() {
            diags.push(
                Diagnostic::error(
                    "ARITY_MISMATCH",
                    self.file,
                    span,
                    &format!("Variant `{}` takes no payload", name),
                )
                .with_types("0 argument(s)", &format!("{}", args.len())),
            );
            for arg in args {
                self.check_expr(arg, diags);
            }
        }
    }

    fn check_method_call(
        &mut self,
        receiver: &Expr,
        method: &str,
        args: &[Expr],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        let receiver_ty = self.check_expr(receiver, diags);
        match method {
            "map" => self.method_map(receiver_ty, args, span, diags),
            "unwrap" => match receiver_ty {
                Ty::Option(inner) => *inner,
                Ty::Result(ok, _) => *ok,
                Ty::Unknown => Ty::Unknown,
                other => {
                    self.unknown_method(&other, method, args, span, diags);
                    Ty::Unknown
                }
            },
            "unwrap_or" => {
                let (value_ty, _kind) = match receiver_ty {
                    Ty::Option(inner) => (*inner, "Option"),
                    Ty::Result(ok, _) => (*ok, "Result"),
                    Ty::Unknown => {
                        for arg in args {
                            self.check_expr(arg, diags);
                        }
                        return Ty::Unknown;
                    }
                    other => {
                        self.unknown_method(&other, method, args, span, diags);
                        return Ty::Unknown;
                    }
                };
                if args.len() != 1 {
                    diags.push(
                        Diagnostic::error(
                            "ARITY_MISMATCH",
                            self.file,
                            span,
                            "`.unwrap_or` expects exactly one argument",
                        )
                        .with_types("1 argument(s)", &format!("{}", args.len())),
                    );
                    for arg in args {
                        self.check_expr(arg, diags);
                    }
                    return value_ty;
                }
                let actual = self.check_expr(&args[0], diags);
                if !compatible(&value_ty, &actual) {
                    diags.push(
                        Diagnostic::error(
                            "TYPE_MISMATCH",
                            self.file,
                            span,
                            &format!(
                                "`.unwrap_or` default: expected `{}`, found `{}`",
                                value_ty, actual
                            ),
                        )
                        .with_types(&value_ty.to_string(), &actual.to_string())
                        .with_repair(
                            "replace_expression",
                            None,
                            Some(&value_ty.to_string()),
                        ),
                    );
                }
                value_ty
            }
            "is_some" | "is_none" | "is_ok" | "is_err" => {
                let valid = match (&receiver_ty, method) {
                    (Ty::Option(_), "is_some" | "is_none") => true,
                    (Ty::Result(_, _), "is_ok" | "is_err") => true,
                    (Ty::Unknown, _) => true,
                    _ => false,
                };
                if !valid {
                    self.unknown_method(&receiver_ty, method, args, span, diags);
                }
                for arg in args {
                    self.check_expr(arg, diags);
                }
                Ty::Bool
            }
            _ => {
                self.unknown_method(&receiver_ty, method, args, span, diags);
                Ty::Unknown
            }
        }
    }

    fn unknown_method(
        &mut self,
        receiver_ty: &Ty,
        method: &str,
        args: &[Expr],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) {
        diags.push(
            Diagnostic::error(
                "UNKNOWN_METHOD",
                self.file,
                span,
                &format!("Type `{}` has no method `{}`", receiver_ty, method),
            )
            .with_symbol(method)
            .with_repair("rename_symbol", Some(method), None),
        );
        for arg in args {
            self.check_expr(arg, diags);
        }
    }

    fn method_map(
        &mut self,
        receiver_ty: Ty,
        args: &[Expr],
        span: Span,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        let (inner, err_ty) = match receiver_ty {
            Ty::Option(inner) => (*inner, None),
            Ty::Result(ok, err) => (*ok, Some(*err)),
            Ty::Unknown => {
                for arg in args {
                    self.check_expr(arg, diags);
                }
                return Ty::Unknown;
            }
            other => {
                self.unknown_method(&other, "map", args, span, diags);
                return Ty::Unknown;
            }
        };

        if args.len() != 1 {
            diags.push(
                Diagnostic::error(
                    "ARITY_MISMATCH",
                    self.file,
                    span,
                    "`.map` expects exactly one closure argument",
                )
                .with_types("1 argument(s)", &format!("{}", args.len())),
            );
            for arg in args {
                self.check_expr(arg, diags);
            }
            return Ty::Unknown;
        }

        let body_ty = match &args[0] {
            Expr::Lambda { param, body, .. } => self.check_lambda(param, body, inner, diags),
            other => {
                let _ = self.check_expr(other, diags);
                diags.push(
                    Diagnostic::error(
                        "EXPECTED_LAMBDA",
                        self.file,
                        other.span(),
                        "`.map` expects a closure argument, e.g. `|x| expr`",
                    )
                    .with_symbol("map")
                    .with_repair("replace_expression", None, None),
                );
                Ty::Unknown
            }
        };

        match err_ty {
            None => Ty::Option(Box::new(body_ty)),
            Some(err) => Ty::Result(Box::new(body_ty), Box::new(err)),
        }
    }

    fn check_lambda(
        &mut self,
        param: &str,
        body: &Expr,
        param_ty: Ty,
        diags: &mut Vec<Diagnostic>,
    ) -> Ty {
        self.scopes.push(HashMap::new());
        self.scopes
            .last_mut()
            .unwrap()
            .insert(param.to_string(), param_ty);
        let body_ty = self.check_expr(body, diags);
        self.scopes.pop();
        body_ty
    }

    fn check_try(&mut self, expr: &Expr, span: Span, diags: &mut Vec<Diagnostic>) -> Ty {
        let inner = self.check_expr(expr, diags);
        match inner {
            Ty::Result(ok, err) => {
                match &self.current_return {
                    Ty::Result(_, current_err) => {
                        if !compatible(current_err, &err) {
                            diags.push(
                                Diagnostic::error(
                                    "TRY_TYPE_MISMATCH",
                                    self.file,
                                    span,
                                    &format!(
                                        "`?` propagates error `{}` but the function returns `{}`",
                                        err, self.current_return
                                    ),
                                )
                                .with_types(&current_err.to_string(), &err.to_string()),
                            );
                        }
                    }
                    _ => {
                        diags.push(Diagnostic::error(
                            "INVALID_TRY",
                            self.file,
                            span,
                            &format!(
                                "`?` on a `Result` requires the enclosing function to return `Result`, found `{}`",
                                self.current_return
                            ),
                        ));
                    }
                }
                *ok
            }
            Ty::Option(value) => {
                if !matches!(self.current_return, Ty::Option(_) | Ty::Unknown) {
                    diags.push(Diagnostic::error(
                        "INVALID_TRY",
                        self.file,
                        span,
                        &format!(
                            "`?` on an `Option` requires the enclosing function to return `Option`, found `{}`",
                            self.current_return
                        ),
                    ));
                }
                *value
            }
            Ty::Unknown => Ty::Unknown,
            other => {
                diags.push(Diagnostic::error(
                    "INVALID_TRY",
                    self.file,
                    span,
                    &format!("`?` requires an `Option` or `Result`, found `{}`", other),
                ));
                Ty::Unknown
            }
        }
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

/// Structural compatibility where `Unknown` acts as a wildcard. This lets
/// constructors like `None` / `Err(e)` flow into more specific expected types
/// without false positives.
fn compatible(a: &Ty, b: &Ty) -> bool {
    match (a, b) {
        (Ty::Unknown, _) | (_, Ty::Unknown) => true,
        (Ty::Array(x), Ty::Array(y)) => compatible(x, y),
        (Ty::Map(k1, v1), Ty::Map(k2, v2)) => compatible(k1, k2) && compatible(v1, v2),
        (Ty::Option(x), Ty::Option(y)) => compatible(x, y),
        (Ty::Result(a1, a2), Ty::Result(b1, b2)) => compatible(a1, b1) && compatible(a2, b2),
        _ => a == b,
    }
}
