//! AgentLang compiler library.
//!
//! Phase 1 walking skeleton: source → tokens → AST → type check → JSON diagnostics.

pub mod ast;
pub mod bench;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod typecheck;
pub mod types;

use diagnostic::Diagnostic;

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
