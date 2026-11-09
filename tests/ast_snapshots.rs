//! Golden-file snapshot tests for the parser: source → pretty-printed AST.
//!
//! Each `tests/snapshots/<name>.ag` input has a sibling `<name>.ast` snapshot
//! produced by `Debug` pretty-printing the parsed `Module`. Spans are included,
//! so the snapshots also pin down source locations.
//!
//! To regenerate the snapshots after an intentional parser change:
//!
//! ```text
//! AGENT_BLESS=1 cargo test --test ast_snapshots
//! ```
//!
//! (PowerShell: `$env:AGENT_BLESS='1'; cargo test --test ast_snapshots`.)

use std::fs;
use std::path::{Path, PathBuf};

use agentlang::lexer::Lexer;
use agentlang::parser::Parser;

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
}

fn is_ag(path: &Path) -> bool {
    path.extension().map(|ext| ext == "ag").unwrap_or(false)
}

fn normalize(source: &str) -> String {
    source.replace("\r\n", "\n")
}

/// Parse `source` and render the module as a stable pretty-printed tree.
fn render_ast(source: &str, file: &str) -> Result<String, String> {
    let mut lexer = Lexer::new(source, file);
    let tokens = lexer
        .tokenize()
        .map_err(|diag| format!("lex error: {}", diag.message))?;

    let mut parser = Parser::new(tokens, file);
    let (module, parse_diags) = parser.parse_module();
    if !parse_diags.is_empty() {
        let messages: Vec<String> = parse_diags
            .iter()
            .map(|d| format!("{}:{}: {}", d.line, d.col, d.message))
            .collect();
        return Err(format!("parse errors:\n  {}", messages.join("\n  ")));
    }

    Ok(format!("{module:#?}\n"))
}

fn first_difference(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    for index in 0..expected_lines.len().max(actual_lines.len()) {
        let expected_line = expected_lines.get(index).copied().unwrap_or("<missing>");
        let actual_line = actual_lines.get(index).copied().unwrap_or("<missing>");
        if expected_line != actual_line {
            return format!(
                "first difference at line {}:\n  expected: {}\n  actual:   {}",
                index + 1,
                expected_line,
                actual_line
            );
        }
    }
    "snapshots differ but no line difference was found".to_string()
}

#[test]
fn ast_snapshots_match() {
    let dir = snapshot_dir();
    let bless = std::env::var("AGENT_BLESS")
        .map(|value| value == "1")
        .unwrap_or(false);

    let mut inputs: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {}", dir.display(), err))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| is_ag(path))
        .collect();
    inputs.sort();
    assert!(
        !inputs.is_empty(),
        "no `.ag` snapshot inputs found in {}",
        dir.display()
    );

    let mut failures: Vec<String> = Vec::new();

    for input in &inputs {
        let name = input
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();
        let source = match fs::read_to_string(input) {
            Ok(source) => normalize(&source),
            Err(err) => {
                failures.push(format!("{}: cannot read input: {}", name, err));
                continue;
            }
        };

        let rendered = match render_ast(&source, &name) {
            Ok(rendered) => rendered,
            Err(err) => {
                failures.push(format!("{}: {}", name, err));
                continue;
            }
        };

        let snapshot_path = input.with_extension("ast");

        if bless {
            if let Err(err) = fs::write(&snapshot_path, &rendered) {
                failures.push(format!("{}: cannot write snapshot: {}", name, err));
            }
            continue;
        }

        match fs::read_to_string(&snapshot_path) {
            Ok(expected) => {
                let expected = normalize(&expected);
                if expected != rendered {
                    failures.push(format!(
                        "{}: snapshot mismatch ({}).\n{}",
                        name,
                        snapshot_path.display(),
                        first_difference(&expected, &rendered)
                    ));
                }
            }
            Err(_) => failures.push(format!(
                "{}: missing snapshot {}. Regenerate with AGENT_BLESS=1 cargo test --test ast_snapshots",
                name,
                snapshot_path.display()
            )),
        }
    }

    if bless {
        assert!(
            failures.is_empty(),
            "failed to write snapshots:\n{}",
            failures.join("\n")
        );
        return;
    }

    assert!(
        failures.is_empty(),
        "AST snapshot failures:\n\n{}\n\nTo update snapshots intentionally: AGENT_BLESS=1 cargo test --test ast_snapshots",
        failures.join("\n\n")
    );
}

#[test]
fn every_snapshot_has_an_input() {
    let dir = snapshot_dir();
    let mut orphans: Vec<String> = Vec::new();

    for entry in fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("cannot read {}: {}", dir.display(), err))
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();
        if path.extension().map(|ext| ext == "ast").unwrap_or(false) {
            let input = path.with_extension("ag");
            if !input.exists() {
                orphans.push(path.display().to_string());
            }
        }
    }

    orphans.sort();
    assert!(
        orphans.is_empty(),
        "found `.ast` snapshots without a matching `.ag` input: {}",
        orphans.join(", ")
    );
}
