//! Code generation: lower AgentLang AST to Rust source.
//!
//! Phase 2: takes a verified `Module` (already type-checked) and emits idiomatic
//! Rust source that `rustc` can compile. Every AgentLang construct has a direct
//! Rust equivalent, so the translation is mostly syntactic with type mapping.

use crate::ast::*;

/// A Rust source code generator with indentation tracking.
pub struct Codegen {
    output: String,
    indent: usize,
    in_main: bool,
    main_returns_unit: bool,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            output: String::new(),
            indent: 0,
            in_main: false,
            main_returns_unit: true,
        }
    }

    pub fn into_output(self) -> String {
        self.output
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
    }

    fn writeln(&mut self, s: &str) {
        for line in s.lines() {
            self.output.push_str(&self.indent_str());
            self.output.push_str(line);
            self.output.push('\n');
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    // -----------------------------------------------------------------------
    // Module-level items
    // -----------------------------------------------------------------------

    /// Lower an entire module to Rust source.
    pub fn lower_module(&mut self, module: &Module) {
        // Emit AgentLang runtime prelude first.
        self.writeln("// --- AgentLang runtime prelude ---");
        self.writeln("use std::collections::HashMap;");
        self.writeln("");
        self.writeln("/// Bounds-checked read of a `Vec<T>` using an `i32` index.");
        self.writeln("/// Panics with an AgentLang-style message if the index is out of bounds.");
        self.writeln("pub fn al_index<T>(arr: &Vec<T>, idx: i32) -> &T {");
        self.indent += 1;
        self.writeln("if idx < 0 || idx as usize >= arr.len() {");
        self.indent += 1;
        self.writeln(
            "panic!(\"AgentLang: index out of bounds: {} (length: {})\", idx, arr.len());",
        );
        self.indent -= 1;
        self.writeln("}");
        self.writeln("&arr[idx as usize]");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("/// Bounds-checked mutable read of a `Vec<T>` using an `i32` index.");
        self.writeln("pub fn al_index_mut<T>(arr: &mut Vec<T>, idx: i32) -> &mut T {");
        self.indent += 1;
        self.writeln("if idx < 0 || idx as usize >= arr.len() {");
        self.indent += 1;
        self.writeln(
            "panic!(\"AgentLang: index out of bounds: {} (length: {})\", idx, arr.len());",
        );
        self.indent -= 1;
        self.writeln("}");
        self.writeln("&mut arr[idx as usize]");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("/// Look up a key in a `HashMap<K, V>`. Returns `None` if absent.");
        self.writeln(
            "pub fn al_map_get<'a, K, V>(map: &'a HashMap<K, V>, key: &K) -> Option<&'a V>",
        );
        self.writeln("where");
        self.writeln("    K: std::cmp::Eq + std::hash::Hash,");
        self.writeln("{");
        self.indent += 1;
        self.writeln("map.get(key)");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("/// Insert a key-value pair into a `HashMap<K, V>`.");
        self.writeln(
            "pub fn al_map_insert<K, V>(map: &mut HashMap<K, V>, key: K, val: V) -> Option<V>",
        );
        self.writeln("where");
        self.writeln("    K: std::cmp::Eq + std::hash::Hash,");
        self.writeln("{");
        self.indent += 1;
        self.writeln("map.insert(key, val)");
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.writeln("");

        // Emit type aliases first so they're available for later items.
        for alias in &module.type_aliases {
            let rust_ty = self.type_expr_to_rust(&alias.ty);
            self.writeln(&format!("type {} = {};", alias.name, rust_ty));
        }
        if !module.type_aliases.is_empty() {
            self.writeln("");
        }

        // Emit structs.
        for decl in &module.structs {
            self.lower_struct(decl);
        }

        // Emit enums.
        for decl in &module.enums {
            self.lower_enum(decl);
        }

        // Emit functions.
        for func in &module.functions {
            self.lower_function(func);
        }
    }

    fn lower_struct(&mut self, decl: &StructDecl) {
        self.writeln("#[derive(Clone, Debug)]");
        self.writeln(&format!("struct {} {{", decl.name));
        self.indent += 1;
        for field in &decl.fields {
            let rust_ty = self.type_expr_to_rust(&field.ty);
            self.writeln(&format!("pub {}: {},", field.name, rust_ty));
        }
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn lower_enum(&mut self, decl: &EnumDecl) {
        self.writeln("#[derive(Clone, Debug)]");
        self.writeln(&format!("enum {} {{", decl.name));
        self.indent += 1;
        for variant in &decl.variants {
            match &variant.payload {
                Some(ty) => {
                    let rust_ty = self.type_expr_to_rust(ty);
                    self.writeln(&format!("{}({}),", variant.name, rust_ty));
                }
                None => {
                    self.writeln(&format!("{},", variant.name));
                }
            }
        }
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
    }

    fn lower_function(&mut self, func: &FunctionDecl) {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, self.type_expr_to_rust(&p.ty)))
            .collect();

        let is_main = func.name == "main";
        let prev_in_main = self.in_main;
        let prev_main_returns_unit = self.main_returns_unit;
        self.in_main = is_main;

        // Determine main's Rust return type based on the declared AgentLang type
        // and whether the body uses the `?` operator.
        let (ret_ty, main_returns_unit, main_needs_exit) = if is_main {
            match func.return_type.name.as_str() {
                "Result" => {
                    // main can return Result<(), E> which implements Termination.
                    let err = func
                        .return_type
                        .args
                        .get(1)
                        .map(|a| self.type_expr_to_rust(a))
                        .unwrap_or_else(|| "String".to_string());
                    let uses_try = self.body_uses_try(&func.body);
                    if uses_try {
                        // main returns Result<(), E>; ? works naturally.
                        (
                            format!("Result<(), {}>", err),
                            false, // main_returns_unit (it's Result)
                            false, // main_needs_exit
                        )
                    } else {
                        (format!("Result<(), {}>", err), false, false)
                    }
                }
                "i32" | "i64" | "i8" | "i16" | "u8" | "u16" | "u32" | "u64" => {
                    // Rust main can't return integers; use process::exit().
                    (
                        "()".to_string(),
                        true,
                        true, // main_needs_exit
                    )
                }
                _ => ("()".to_string(), true, false),
            }
        } else {
            (self.type_expr_to_rust(&func.return_type), true, false)
        };

        self.main_returns_unit = main_returns_unit;
        self.writeln(&format!(
            "fn {}({}) -> {} {{",
            func.name,
            params.join(", "),
            ret_ty
        ));
        self.indent += 1;
        for stmt in &func.body {
            self.lower_stmt(stmt);
        }
        // If main didn't explicitly return, add an implicit terminator.
        if is_main && !self.body_ends_with_return(&func.body) {
            if main_needs_exit {
                self.writeln("std::process::exit(0);");
            } else if !main_returns_unit {
                self.writeln("Ok(())");
            } else {
                self.writeln("return;");
            }
        }
        self.indent -= 1;
        self.writeln("}");
        self.writeln("");
        self.in_main = prev_in_main;
        self.main_returns_unit = prev_main_returns_unit;
    }

    /// Returns true if any statement in the body (recursively) contains a
    /// `?` (Try) expression.
    fn body_uses_try(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            if self.stmt_uses_try(stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_uses_try(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Let { value, .. } => self.expr_uses_try(value),
            Stmt::Assign { value, .. } => self.expr_uses_try(value),
            Stmt::Return { value, .. } => value
                .as_ref()
                .map(|e| self.expr_uses_try(e))
                .unwrap_or(false),
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                then_body.iter().any(|s| self.stmt_uses_try(s))
                    || else_body
                        .as_ref()
                        .map(|b| b.iter().any(|s| self.stmt_uses_try(s)))
                        .unwrap_or(false)
            }
            Stmt::While { body, .. } => body.iter().any(|s| self.stmt_uses_try(s)),
            Stmt::For { body, .. } => body.iter().any(|s| self.stmt_uses_try(s)),
            Stmt::Expr(expr) => self.expr_uses_try(expr),
        }
    }

    fn expr_uses_try(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Try { .. } => true,
            Expr::Binary { left, right, .. } => {
                self.expr_uses_try(left) || self.expr_uses_try(right)
            }
            Expr::Unary { expr, .. } => self.expr_uses_try(expr),
            Expr::Call { args, .. } => args.iter().any(|a| self.expr_uses_try(a)),
            Expr::Member { object, .. } => self.expr_uses_try(object),
            Expr::StructLit { fields, .. } => fields.iter().any(|(_, e)| self.expr_uses_try(e)),
            Expr::Variant { payload, .. } => payload
                .iter()
                .any(|args| args.iter().any(|a| self.expr_uses_try(a))),
            Expr::MethodCall { receiver, args, .. } => {
                self.expr_uses_try(receiver) || args.iter().any(|a| self.expr_uses_try(a))
            }
            Expr::Match {
                scrutinee, arms, ..
            } => self.expr_uses_try(scrutinee) || arms.iter().any(|a| self.expr_uses_try(&a.body)),
            Expr::Index { object, index, .. } => {
                self.expr_uses_try(object) || self.expr_uses_try(index)
            }
            Expr::Group(inner) => self.expr_uses_try(inner),
            _ => false,
        }
    }

    /// Returns true if the last statement in the body is a Return.
    fn body_ends_with_return(&self, body: &[Stmt]) -> bool {
        match body.last() {
            Some(Stmt::Return { .. }) => true,
            _ => false,
        }
    }

    // -----------------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------------

    fn lower_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                value,
                span: _,
            } => {
                let mut_kw = if *mutable { "mut " } else { "" };
                let val = self.expr_to_rust(value);
                self.writeln(&format!("let {}{} = {};", mut_kw, name, val));
            }
            Stmt::Assign {
                target,
                value,
                span: _,
            } => {
                let val = self.expr_to_rust(value);
                match &**target {
                    Expr::Index {
                        object,
                        index,
                        span: _,
                    } => {
                        let obj = self.expr_to_rust(object);
                        let idx = self.expr_to_rust(index);
                        self.writeln(&format!(
                            "*al_index_mut(&mut {}, {} as i32) = {};",
                            obj, idx, val
                        ));
                    }
                    _ => {
                        let tgt = self.expr_to_rust(target);
                        self.writeln(&format!("{} = {};", tgt, val));
                    }
                }
            }
            Stmt::Return { value, span: _ } => match value {
                Some(expr) => {
                    if self.in_main && !self.main_returns_unit {
                        // main returns Result<(), E>; discard the Ok value.
                        self.writeln("return Ok(());");
                    } else if self.in_main && self.main_returns_unit {
                        // main returns () with process::exit(); propagate the exit code.
                        let e = self.expr_to_rust(expr);
                        self.writeln(&format!(
                            "{{
                                let code = {} as i32;
                                std::process::exit(code);
                            }}",
                            e
                        ));
                    } else {
                        let val = self.expr_to_rust(expr);
                        self.writeln(&format!("return {};", val));
                    }
                }
                None => {
                    self.writeln("return;");
                }
            },
            Stmt::If {
                cond,
                then_body,
                else_body,
                span: _,
            } => {
                self.lower_if(cond, then_body, else_body);
            }
            Stmt::While {
                cond,
                body,
                span: _,
            } => {
                let c = self.expr_to_rust(cond);
                self.writeln(&format!("while {} {{", c));
                self.indent += 1;
                for stmt in body {
                    self.lower_stmt(stmt);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            Stmt::For {
                name,
                iterable,
                body,
                span: _,
            } => {
                let iter = self.expr_to_rust(iterable);
                self.writeln(&format!("for {} in {} {{", name, iter));
                self.indent += 1;
                for stmt in body {
                    self.lower_stmt(stmt);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            Stmt::Expr(expr) => {
                let e = self.expr_to_rust(expr);
                self.writeln(&format!("{};", e));
            }
        }
    }

    fn lower_if(&mut self, cond: &Expr, then_body: &[Stmt], else_body: &Option<Vec<Stmt>>) {
        let c = self.expr_to_rust(cond);
        self.writeln(&format!("if {} {{", c));
        self.indent += 1;
        for stmt in then_body {
            self.lower_stmt(stmt);
        }
        self.indent -= 1;
        self.lower_else_tail(else_body);
    }

    fn lower_else_tail(&mut self, else_body: &Option<Vec<Stmt>>) {
        match else_body {
            Some(body) => {
                if body.len() == 1 {
                    if let Stmt::If {
                        cond,
                        then_body,
                        else_body,
                        ..
                    } = &body[0]
                    {
                        let cond_str = self.expr_to_rust(cond);
                        self.write(&format!("}} else if {} {{", cond_str));
                        self.indent += 1;
                        for stmt in then_body {
                            self.lower_stmt(stmt);
                        }
                        self.indent -= 1;
                        self.lower_else_tail(else_body);
                    } else {
                        self.writeln("}");
                        self.writeln("else {");
                        self.indent += 1;
                        for stmt in body {
                            self.lower_stmt(stmt);
                        }
                        self.indent -= 1;
                        self.writeln("}");
                    }
                } else {
                    self.writeln("}");
                    self.writeln("else {");
                    self.indent += 1;
                    for stmt in body {
                        self.lower_stmt(stmt);
                    }
                    self.indent -= 1;
                    self.writeln("}");
                }
            }
            None => {
                self.writeln("}");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------------

    fn expr_to_rust(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Integer(v, _) => v.to_string(),
            Expr::Float(v, _) => self.float_to_rust(*v),
            Expr::Bool(b, _) => b.to_string(),
            Expr::StringLit(s, _) => self.string_lit_to_rust(s),
            Expr::CharLit(c, _) => self.char_lit_to_rust(*c),
            Expr::Ident(n, _) => n.clone(),
            Expr::Binary {
                op, left, right, ..
            } => {
                let l = self.expr_to_rust(left);
                let r = self.expr_to_rust(right);
                let op_str = self.binop_to_rust(*op);
                format!("{} {} {}", l, op_str, r)
            }
            Expr::Unary { op, expr, .. } => {
                let e = self.expr_to_rust(expr);
                let op_str = self.unop_to_rust(*op);
                format!("{}{}", op_str, e)
            }
            Expr::Call { callee, args, .. } => {
                let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_rust(a)).collect();
                format!("{}({})", callee, arg_strs.join(", "))
            }
            Expr::Member { object, field, .. } => {
                let obj = self.expr_to_rust(object);
                format!("{}.{}", obj, field)
            }
            Expr::StructLit { name, fields, .. } => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(fname, fval)| format!("{}: {}", fname, self.expr_to_rust(fval)))
                    .collect();
                format!("{} {{ {} }}", name, field_strs.join(", "))
            }
            Expr::Variant {
                enum_name,
                variant,
                payload,
                ..
            } => match payload {
                Some(args) => {
                    let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_rust(a)).collect();
                    format!("{}::{} ({})", enum_name, variant, arg_strs.join(", "))
                }
                None => format!("{}::{}", enum_name, variant),
            },
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                let recv = self.expr_to_rust(receiver);
                let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_rust(a)).collect();
                format!("{}.{}({})", recv, method, arg_strs.join(", "))
            }
            Expr::Try { expr, .. } => {
                let e = self.expr_to_rust(expr);
                if self.in_main && !self.main_returns_unit {
                    // main returns Result<(), E>; ? must propagate errors but keep the Ok value.
                    // Use match instead of ? to avoid From<i32> for () requirement.
                    format!("match {} {{ Ok(v) => v, Err(e) => return Err(e) }}", e)
                } else {
                    format!("{}?", e)
                }
            }
            Expr::Lambda { param, body, .. } => {
                let b = self.expr_to_rust(body);
                format!("|{}| {}", param, b)
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                let scrutinee_str = self.expr_to_rust(scrutinee);
                let mut arm_strs = Vec::new();
                for arm in arms {
                    let pat_str = self.pattern_to_rust(&arm.pattern);
                    let body_str = self.expr_to_rust(&arm.body);
                    arm_strs.push(format!("{} => {}", pat_str, body_str));
                }
                format!("match {} {{ {} }}", scrutinee_str, arm_strs.join(", "))
            }
            Expr::Index { object, index, .. } => {
                let obj = self.expr_to_rust(object);
                let idx = self.expr_to_rust(index);
                format!("*al_index(&{}, {} as i32)", obj, idx)
            }
            Expr::Group(inner) => {
                let e = self.expr_to_rust(inner);
                format!("({})", e)
            }
        }
    }

    fn pattern_to_rust(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Wildcard { .. } => "_".to_string(),
            Pattern::Variant {
                enum_name,
                variant,
                has_parens,
                binding,
                ..
            } => {
                let qualified = match enum_name {
                    Some(en) => format!("{}::", en),
                    None => String::new(),
                };
                match binding {
                    Some(name) if name == "_" => {
                        if *has_parens {
                            format!("{}_({})", qualified, variant)
                        } else {
                            format!("{}_{}", qualified, variant)
                        }
                    }
                    Some(name) => {
                        if *has_parens {
                            format!("{}{}({})", qualified, variant, name)
                        } else {
                            format!("{} {}", qualified, variant)
                        }
                    }
                    None => format!("{}{}", qualified, variant),
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Type expressions
    // -----------------------------------------------------------------------

    fn type_expr_to_rust(&self, ty: &TypeExpr) -> String {
        match ty.name.as_str() {
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64"
            | "Bool" | "String" | "Char" => ty.name.clone(),
            "Unit" => "()".to_string(),
            "Array" => {
                let inner = ty
                    .args
                    .first()
                    .map(|a| self.type_expr_to_rust(a))
                    .unwrap_or_else(|| "Unknown".to_string());
                format!("Vec<{}>", inner)
            }
            "Map" => {
                let k = ty
                    .args
                    .first()
                    .map(|a| self.type_expr_to_rust(a))
                    .unwrap_or_else(|| "Unknown".to_string());
                let v = ty
                    .args
                    .get(1)
                    .map(|a| self.type_expr_to_rust(a))
                    .unwrap_or_else(|| "Unknown".to_string());
                format!("std::collections::HashMap<{}, {}>", k, v)
            }
            "Option" => {
                let inner = ty
                    .args
                    .first()
                    .map(|a| self.type_expr_to_rust(a))
                    .unwrap_or_else(|| "Unknown".to_string());
                format!("Option<{}>", inner)
            }
            "Result" => {
                let ok = ty
                    .args
                    .first()
                    .map(|a| self.type_expr_to_rust(a))
                    .unwrap_or_else(|| "Unknown".to_string());
                let err = ty
                    .args
                    .get(1)
                    .map(|a| self.type_expr_to_rust(a))
                    .unwrap_or_else(|| "Unknown".to_string());
                format!("Result<{}, {}>", ok, err)
            }
            _ => ty.name.clone(), // user-defined type
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn binop_to_rust(&self, op: BinOp) -> &'static str {
        match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }

    fn unop_to_rust(&self, op: UnOp) -> &'static str {
        match op {
            UnOp::Neg => "-",
            UnOp::Not => "!",
        }
    }

    fn float_to_rust(&self, v: f64) -> String {
        let s = v.to_string();
        if !s.contains('.')
            && !s.contains('e')
            && !s.contains('E')
            && !s.contains("inf")
            && !s.contains("nan")
        {
            format!("{}.0", s)
        } else {
            s
        }
    }

    fn string_lit_to_rust(&self, s: &str) -> String {
        let mut out = String::from("String::from(\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_control() => out.push_str(&format!("\\{:03o}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('\"');
        out.push(')');
        out
    }

    fn char_lit_to_rust(&self, c: char) -> String {
        format!("'{}'", c)
    }
}

impl Default for Codegen {
    fn default() -> Self {
        Self::new()
    }
}
