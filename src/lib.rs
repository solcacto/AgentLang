//! AgentLang compiler library.
//!
//! Phase 1 walking skeleton: source → tokens → AST → type check → JSON diagnostics.
//! Phase 2: codegen lowers a verified Module to Rust source.

pub mod ast;
pub mod bench;
pub mod codegen;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod rt;
pub mod typecheck;
pub mod types;

use crate::lexer::Span;
use ast::Module;
use diagnostic::Diagnostic;
use std::collections::HashMap;
use types::Ty;

/// Run the full front-end on a source string.
///
/// Returns an empty vector when the program is valid. On failure, returns
/// structured diagnostics suitable for an agent to consume and self-repair.
pub fn check_source(source: &str, file: &str) -> Vec<Diagnostic> {
    let mut lexer = lexer::Lexer::new(source, file);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(diag) => return vec![diag],
    };

    let mut parser = parser::Parser::new(tokens, file);
    let (module, parse_diags) = parser.parse_module();

    // If the file did not parse cleanly, report the parser diagnostics and skip
    // type checking: a partial AST would only produce spurious follow-on errors.
    if !parse_diags.is_empty() {
        return parse_diags;
    }

    let mut checker = typecheck::TypeChecker::new(file);
    checker.check_module(&module)
}

/// Run the full front-end and return the verified `Module` on success.
///
/// This is used by `agent build` and `agent run` to lower a type-checked
/// program to Rust. Returns the module plus resolved expression types
/// (keyed by span) for codegen.
pub fn check_and_lower(
    source: &str,
    file: &str,
) -> Result<(Module, HashMap<Span, Ty>), Vec<Diagnostic>> {
    let mut lexer = lexer::Lexer::new(source, file);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(diag) => return Err(vec![diag]),
    };

    let mut parser = parser::Parser::new(tokens, file);
    let (module, parse_diags) = parser.parse_module();

    if !parse_diags.is_empty() {
        return Err(parse_diags);
    }

    let mut checker = typecheck::TypeChecker::new(file);
    let diags = checker.check_module(&module);

    if diags.is_empty() {
        Ok((module, checker.resolved_types))
    } else {
        Err(diags)
    }
}
