# Phase 1 — Language Core: Todo List

> Goal: a small but fully functional compiler front-end that turns `.ag` source into a verified AST with machine-readable JSON diagnostics.
>
> **Status:** Walking skeleton is working. `agent check` lexes, parses, and type-checks functions, structs, enums, `Option`/`Result`, `?`, minimal method calls, `match`, `if`/`else`, `let mut` assignment, and `while` loops — emitting structured JSON diagnostics. AST snapshots and a self-repair benchmark harness are in place. See `src/` and `tests/`.

**Definition of done:** The `agent check` command parses and type-checks a `.ag` file, and on failure emits a structured JSON diagnostic with a repair suggestion. Success is measured by an LLM being able to read the JSON, fix the code, and re-run to a clean check — without a human reading compiler output.

---

## Milestone 0 — Foundation & Scaffolding

- [x] **Decide implementation language** (Rust)
- [x] Create repo layout (`src/`, `tests/`, `examples/`, `docs/`)
- [x] Set up `Cargo.toml`
- [x] Add CI workflow (build + test on push)
- [x] Add `.gitignore`
- [x] Choose CLI framework (hand-rolled arg parsing in `src/main.rs`)
- [x] Write `CONTRIBUTING.md` with build/test instructions
- [x] Define the `.ag` file extension and `examples/hello.ag`

## Milestone 1 — Lexer (Tokenizer)

- [x] Define `Token` enum (keywords, identifiers, literals, operators, punctuation)
- [x] Define `Span` (line/column) for precise diagnostics
- [x] Skip whitespace and handle `//` and `/* */` comments
- [x] Lex integer, float, string, boolean, and char literals
- [x] Lex identifiers and keywords (`function`, `type`, `struct`, `enum`, `let`, `return`, …)
- [x] Lex multi-char operators (`->`, `==`, `>=`, `?`, `|`, …)
- [x] Emit diagnostic on invalid/unterminated tokens
- [x] Unit/integration tests: valid token streams + error cases

## Milestone 2 — AST & Parser

- [x] Define AST node types (`Module`, `FunctionDecl`, `Param`, `TypeExpr`, `Stmt`, `Expr`, …)
- [x] Implement recursive-descent parser skeleton
- [x] Parse function signatures (`function name(params) -> Type effects(...)`)
- [x] Parse function bodies and `return` statements
- [x] Parse `let` bindings
- [x] Parse expressions with correct precedence (binary, unary, calls, member access)
- [x] Parse type expressions (named, generic like `Result<T, E>`, `Array<T>`)
- [x] Parse `struct` and `enum` declarations
- [x] Parse `Enum::Variant` paths, method calls, closures, and the `?` operator
- [x] Parse `if` / `else` statements, including `else if` and nested blocks
- [x] Parse `let mut` bindings, assignment statements, and `while` loops
- [x] Error recovery: report a diagnostic and recover to the next declaration/statement
- [x] Snapshot tests: source → AST (`tests/snapshots/`, see `tests/ast_snapshots.rs`)

## Milestone 3 — Static Type Checking

- [x] Define type representation (`i32`, `f64`, `Bool`, `String`, `Array<T>`, `Map<K,V>`, user types)
- [x] Build symbol table / scoped environment
- [x] Resolve type names (including forward references within a module)
- [x] Type-check `let` bindings (literal inference; no implicit conversions)
- [x] Type-check expressions and operators
- [x] Type-check function calls (arity + argument types + return type)
- [x] Type-check `return` against declared return type
- [x] Type-check `if` / `else`: `Bool` conditions, per-branch scopes, and return-path coverage
- [x] Enforce mutability on assignment (`IMMUTABLE_ASSIGNMENT`) and type-check `while` conditions
- [x] Detect undefined symbols and duplicate declarations
- [x] Unit tests: accept valid programs, reject invalid ones with correct error codes

## Milestone 4 — Structs, Enums & Functions

- [x] Register struct definitions and their fields
- [x] Register enum definitions and variants
- [x] Type-check struct literal construction and field access
- [x] Type-check enum variant construction (`Enum::Variant`, with payloads)
- [x] Basic `match` on enums with payload binding, wildcard arms, and exhaustiveness checks
- [x] Type-check function declarations end-to-end
- [x] Integrate with symbol table and type resolution

## Milestone 5 — `Result` and `Option`

- [x] Add `Option<T>` and `Result<T, E>` as built-in generic types
- [x] Implement `Option`/`Result` construction (`Some`, `None`, `Ok`, `Err`, and qualified `Option::*` / `Result::*`)
- [x] Type-check `.map(...)` method calls (closures supported as `map` arguments)
- [x] Implement `?` operator propagation with correct error-type checking
- [x] Enforce explicit handling (no implicit unwrap; `unwrap`/`unwrap_or` are explicit)
- [x] Unit tests for success and error propagation paths

## Milestone 6 — JSON Compiler Diagnostics

- [x] Define `Diagnostic` struct: `code`, `severity`, `file`, `line`, `col`, `symbol`, `expected`, `received`, `message`, `repair`, `repairable`, `affects_other_files`
- [x] Assign stable error codes (`TYPE_MISMATCH`, `UNDEFINED_SYMBOL`, `ARITY_MISMATCH`, `PARSE_ERROR`, …)
- [x] Populate `expected`/`received` from type-checker failures
- [x] Add `repair` suggestions (e.g., `replace_expression`, `add_import`)
- [x] Serialize diagnostics to JSON (stable output schema; see `docs/diagnostics.md`)
- [x] Human-readable fallback renderer for local dev (`message` field)
- [x] Golden/snapshot tests for the JSON schema

## Milestone 7 — CLI & Integration

- [x] Implement `agent check <file>` → parse + type-check → exit code + JSON diagnostics
- [x] Implement `agent build` stub (compiles to a no-op / IR placeholder for now)
- [x] `agent check` always emits JSON on failure
- [x] End-to-end test: broken `.ag` file → JSON → simulated fix → clean check
- [x] Write a self-repair benchmark harness (`agent bench`) with a pluggable `Repairer` trait
- [x] Provider-agnostic `CommandRepairer` to drive any agent harness/CLI (AgentLang holds no API key)
- [x] Machine-mode `agent check --format json` contract (always JSON, `[]` on success)
- [ ] Record a real model-driven benchmark run (point `--repair-cmd` at a harness/model)

---

## Phase 1 Exit Criteria

- [ ] All above milestones complete
- [x] A first-attempt-compile benchmark baseline recorded (see "Baseline" below)
- [x] `agent check` usable by an autonomous agent with zero human intervention
- [x] Documented JSON diagnostic schema in `docs/diagnostics.md`

---

## Baseline (recorded 2026-11-09)

`agent bench benchmarks` over thirteen tasks (`benchmarks/01_ok_baseline` … `13_assignment_type_mismatch`):

| Metric | Value |
| --- | --- |
| Tasks | 13 |
| First-pass clean | 1 / 13 (the intentional baseline task) |
| Resolved with oracle repair | 13 / 13 |
| Mean repair iterations (non-clean tasks) | 1.0 |
| Remaining unresolved | 0 |

The oracle repairer is an upper bound: it proves the harness loop converges and
that every broken task is genuinely repairable from its diagnostics. The next
measurement is the same corpus driven by a real LLM `Repairer`, which will yield
the meaningful first-attempt-compile and iterations-to-green numbers.

---

## Suggested Build Order

Do **vertical slices**, not horizontal layers. Get one thing working end-to-end before adding breadth:

1. **Skeleton:** `function main() -> i32 { return 0; }` parses and type-checks clean. ✅
2. **One error path:** a type mismatch produces valid JSON diagnostics. ✅
3. **Grow the grammar** (let, expressions, calls) behind the same diagnostic pipeline. ✅
4. **Add structs/enums**, then `Result`/`Option`, then `?`, then `match`. ✅
5. **Harden** with error recovery, AST snapshots, and the self-repair benchmark harness. ✅
6. **Close the loop:** implement a real LLM `Repairer` and record the first-attempt-compile baseline. ← next

This keeps the differentiator (structured diagnostics) visible from day one instead of bolting it on at the end.
