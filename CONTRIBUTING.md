# Contributing to AgentLang

Thanks for your interest! This is an early-stage compiler. Contributions that
keep the agent-first design in focus are welcome.

## Prerequisites

- [Rust](https://rustup.rs) (stable toolchain)

## Build and test

```bash
cargo build
cargo test
```

## Trying the CLI

```bash
cargo run -- check examples/hello.ag   # OK
cargo run -- check examples/bad.ag     # JSON diagnostics, exit code 1
cargo run -- check examples/structs.ag # OK
```

## Code layout

| File | Responsibility |
|------|----------------|
| `src/lexer.rs` | Tokenizer with source spans |
| `src/parser.rs` | Recursive-descent parser |
| `src/ast.rs` | Abstract syntax tree |
| `src/types.rs` | Semantic type representation |
| `src/typecheck.rs` | Static type checker |
| `src/diagnostic.rs` | Structured JSON diagnostics |
| `src/bench.rs` | Self-repair benchmark harness |
| `docs/diagnostics.md` | Diagnostic schema contract |

## AST snapshot tests

`tests/snapshots/<name>.ag` inputs are parsed and compared against a golden
`<name>.ast` pretty-printed AST (including source spans). Add an input file to
cover a new grammar construct.

If a parser change alters the AST intentionally, regenerate the snapshots:

```bash
# Unix
AGENT_BLESS=1 cargo test --test ast_snapshots

# PowerShell
$env:AGENT_BLESS='1'; cargo test --test ast_snapshots; Remove-Item Env:\AGENT_BLESS
```

Review the resulting `.ast` diff before committing — snapshots are a contract,
not an automatic output.

## Guidelines

- **Diagnostics are the product.** Every user-facing error must be structured
  JSON with a stable `code`, and where possible a `repair`. Never free-text only.
- Add a test in `tests/integration.rs` for each new feature or diagnostic.
- Prefer vertical slices: make one construct work end-to-end before broadening.
- Run `cargo fmt` and `cargo test` before committing.

## Roadmap

See [`PHASE1_TODO.md`](PHASE1_TODO.md) for current priorities.
