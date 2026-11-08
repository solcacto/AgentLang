# AgentLang

**AgentLang** is a proposed programming language designed around a simple premise: the primary programmer is an advanced LLM (autonomous coding agent), not a human.

Most existing languages were designed around human limitations (typo-prone syntax, need for readable abstractions, slow at repetition, etc.). AgentLang assumes code will be generated, modified, tested, and maintained by autonomous agents.

## Core Idea

A statically typed, memory-safe, compiled language with explicit permissions, machine-readable diagnostics, incremental verification, and deterministic behaviour — built specifically for machine-generated software.

## Key Features

| Feature | Description |
|---------|-------------|
| **Capability-based security** | Every function declares what it can access; no implicit permissions |
| **Structured diagnostics** | Compiler returns machine-readable JSON errors with repair suggestions |
| **Incremental compilation** | Live Verification Engine analyses only changed code and affected dependencies |
| **Built-in contracts** | `requires` / `ensures` with optional SMT solver verification |
| **Memory safety** | Automatic management by default, ownership/borrowing where needed |
| **Native concurrency** | First-class `parallel` blocks, structured concurrency, race detection |
| **Structured code editing** | Agents modify functions via AST operations (create/modify/replace), not raw text |

## Development Roadmap

The project is being built in five phases. Start with a small, fully functional compiler and toolchain:

- **Phase 1 — Language core:** Lexer, parser, static typing, structs/enums/functions, `Result`/`Option`, JSON diagnostics
- **Phase 2 — Agent toolchain:** Structured AST interface, function-level modification, incremental compilation, dependency tracking
- **Phase 3 — Security:** Capability permissions, memory safety, effect tracking, sandboxed execution
- **Phase 4 — Production compiler:** LLVM backend, native executables, WASM backend, standard library, package manager
- **Phase 5 — Verification ecosystem:** Contract system, SMT integration, automatic test generation, change impact analysis

## Building

Requires a stable Rust toolchain.

```bash
cargo build
cargo test
```

## Usage

The Phase 1 walking skeleton provides `agent check`, which lexes, parses, type-checks, and emits structured JSON diagnostics:

```bash
# A valid program
cargo run -- check examples/hello.ag
# OK: examples/hello.ag passed all checks

# A program with a type error → JSON diagnostics on stdout, exit code 1
cargo run -- check examples/bad.ag

# Structs and field access
cargo run -- check examples/structs.ag

# Option/Result, the ? operator, and methods
cargo run -- check examples/option.ag
cargo run -- check examples/result.ag
cargo run -- check examples/map.ag
```

### Phase 1 language subset

Currently supported: functions with typed parameters and `effects(...)`, `let`,
`return`, arithmetic/comparison/logical operators, function calls, `struct`
declarations with typed fields, struct literals, field access, `enum`
declarations and `Enum::Variant` construction (with payloads), built-in
`Option<T>`/`Result<T, E>` construction (`Some`/`None`/`Ok`/`Err` and
`Option::*`/`Result::*`), the `?` operator, closures as `.map` arguments, and
the methods `map`, `unwrap`, `unwrap_or`, `is_some`, `is_none`, `is_ok`,
`is_err`. Control flow and `match` are upcoming (see
[`PHASE1_TODO.md`](PHASE1_TODO.md)).

Example diagnostic output:

```json
[
  {
    "severity": "error",
    "code": "TYPE_MISMATCH",
    "file": "examples/bad.ag",
    "line": 4,
    "col": 5,
    "expected": "i32",
    "received": "String",
    "message": "Return type mismatch: expected `i32`, found `String`",
    "repair": { "type": "replace_expression", "expected_type": "i32" },
    "repairable": true,
    "affects_other_files": false
  }
]
```

See [`docs/diagnostics.md`](docs/diagnostics.md) for the diagnostic schema and [`PHASE1_TODO.md`](PHASE1_TODO.md) for current progress.

## Documentation

- [`Overview.md`](Overview.md) — full design document (features, CLI commands, architecture, priorities)
- [`Overview.txt`](Overview.txt) — original design text
- [`PHASE1_TODO.md`](PHASE1_TODO.md) — Phase 1 task list and status
- [`docs/diagnostics.md`](docs/diagnostics.md) — machine-readable diagnostic contract

## Contributing

This is an early-stage design document. The four key architectural decisions are still open:

1. **Primary use case:** Agentic coding (leaning)
2. **How LLMs write code:** Hybrid human-readable + structured interface (leaning)
3. **Verification strictness:** Balanced (leaning)
4. **Compilation targets:** Native and WASM equally (leaning)

Open an issue to discuss any of these.

## License

Not yet specified.