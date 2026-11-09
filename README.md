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

# Pattern matching on enums, Option, and Result
cargo run -- check examples/match.ag

# if / else statements, including else-if chains
cargo run -- check examples/control_flow.ag

# Mutable bindings (let mut), assignment, and while loops
cargo run -- check examples/loops.ag

# for loops and array/map indexing
cargo run -- check examples/collections.ag

# Field and index mutation (requires let mut)
cargo run -- check examples/mutation.ag

# Machine mode: always emit a JSON array (empty `[]` on success)
cargo run -- check --format json examples/hello.ag
cargo run -- check --json examples/bad.ag

# Self-repair benchmark suite (oracle baseline by default)
cargo run -- bench
cargo run -- bench benchmarks --repairer none --max-iters 3

# Drive the benchmark through any external agent/harness command
cargo run -- bench benchmarks --repairer command --repair-cmd "my-agent --file {source}"
```

### Phase 1 language subset

Currently supported: functions with typed parameters and `effects(...)`, `let`,
`return`, arithmetic/comparison/logical operators, function calls, `struct`
declarations with typed fields, struct literals, field access, `enum`
declarations and `Enum::Variant` construction (with payloads), built-in
`Option<T>`/`Result<T, E>` construction (`Some`/`None`/`Ok`/`Err` and
`Option::*`/`Result::*`), the `?` operator, closures as `.map` arguments, the
methods `map`, `unwrap`, `unwrap_or`, `is_some`, `is_none`, `is_ok`, `is_err`,
`match` over enums/`Option`/`Result` (with payload binding, wildcards, and
exhaustiveness checking), `if` / `else if` / `else` statements, `let mut`
bindings with assignment, and `while` loops — all with block scoping and
return-path checking. `for name in iterable { ... }` loops iterate over
`Array<T>` (binding `T`) and `Map<K, V>` (binding `K`); `expr[index]` indexes
arrays with `i32` and maps with `K`. Assignment targets may be variables, struct fields (`point.x`), or index expressions (`items[0]`); the root binding must be declared with `let mut`.

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

See [`docs/diagnostics.md`](docs/diagnostics.md) for the diagnostic schema, [`docs/benchmarks.md`](docs/benchmarks.md) for the self-repair harness, and [`PHASE1_TODO.md`](PHASE1_TODO.md) for current progress.

## Self-repair benchmarks

The core thesis is that an agent can read the JSON diagnostics, repair the
program, and converge to a clean check without human help. `agent bench` makes
that measurable: each task is a broken `.ag` program plus a known-good version,
and a pluggable `Repairer` proposes fixes until the check passes.

```bash
cargo run -- bench benchmarks
```

The default `OracleRepairer` returns the known-good source, which validates the
loop and records an upper-bound baseline. `--repairer none` runs first-pass only.

For real runs, use `CommandRepairer` to plug in **any** agent harness or CLI.
AgentLang never holds an API key — the harness owns its model and credentials:

```bash
cargo run -- bench benchmarks \
  --repairer command \
  --repair-cmd "my-agent --file {source}"
```

The command receives the current source at `{source}`, the diagnostics at
`{diagnostics}`, and a scratch directory at `{dir}` (also available via
`AGENTLANG_*` environment variables). It edits `source.ag` in place; the harness
re-checks whatever comes back. See [`docs/benchmarks.md`](docs/benchmarks.md).

Reports include a `metadata` block (`tool`, `version`, `repairer`, `max_iters`)
so results can be tied to a specific build, repairer, and iteration budget.

## Documentation

- [`Overview.md`](Overview.md) — full design document (features, CLI commands, architecture, priorities)
- [`Overview.txt`](Overview.txt) — original design text
- [`PHASE1_TODO.md`](PHASE1_TODO.md) — Phase 1 task list and status
- [`docs/diagnostics.md`](docs/diagnostics.md) — machine-readable diagnostic contract
- [`docs/benchmarks.md`](docs/benchmarks.md) — self-repair benchmark harness and how to add tasks

## Contributing

This is an early-stage design document. The four key architectural decisions are still open:

1. **Primary use case:** Agentic coding (leaning)
2. **How LLMs write code:** Hybrid human-readable + structured interface (leaning)
3. **Verification strictness:** Balanced (leaning)
4. **Compilation targets:** Native and WASM equally (leaning)

Open an issue to discuss any of these.

## License

MIT — see [`LICENSE`](LICENSE).