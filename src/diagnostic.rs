//! Structured, machine-readable compiler diagnostics.
//!
//! This is the core differentiator of AgentLang: the compiler speaks to agents,
//! not just humans. Every diagnostic carries enough information for an LLM to
//! locate and repair the problem without parsing free-text compiler output.

use crate::lexer::Span;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Repair {
    /// The kind of repair, e.g. `replace_expression`, `add_import`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair: Option<Repair>,
    pub repairable: bool,
    pub affects_other_files: bool,
}

impl Diagnostic {
    pub fn error(code: &str, file: &str, span: Span, message: &str) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code: code.to_string(),
            file: file.to_string(),
            line: span.line,
            col: span.col,
            symbol: None,
            expected: None,
            received: None,
            message: message.to_string(),
            repair: None,
            repairable: false,
            affects_other_files: false,
        }
    }

    pub fn with_symbol(mut self, symbol: &str) -> Self {
        self.symbol = Some(symbol.to_string());
        self
    }

    pub fn with_types(mut self, expected: &str, received: &str) -> Self {
        self.expected = Some(expected.to_string());
        self.received = Some(received.to_string());
        self
    }

    pub fn with_repair(
        mut self,
        kind: &str,
        target: Option<&str>,
        expected_type: Option<&str>,
    ) -> Self {
        self.repair = Some(Repair {
            kind: kind.to_string(),
            target: target.map(|s| s.to_string()),
            expected_type: expected_type.map(|s| s.to_string()),
        });
        self.repairable = true;
        self
    }
}
