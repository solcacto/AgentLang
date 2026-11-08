# Phase 1 — Language Core: Todo List

> Goal: a small but fully functional compiler front-end that turns `.ag` source into a verified AST with machine-readable JSON diagnostics.

**Definition of done:** The `agent check` command parses and type-checks a `.ag` file, and on failure emits a structured JSON diagnostic with a repair suggestion. Success is measured by an LLM being able to read the JSON, fix the code, and re-run to a clean check — without a human reading compiler output.

---

## Milestone 0 — Foundation & Scaffolding

- [ ] **Decide implementation language** (recommendation: Rust)
- [ ] Create repo layout (`src/`, `tests/`, `examples/`, `docs/`)
- [ ] Set up `Cargo.toml` / project manifest
- [ ] Add CI workflow (build + test on push)
- [ ] Add `.gitignore`
- [ ] Choose CLI framework (e.g., `clap`)
- [ ] Write `CONTRIBUTING.md` with build/test instructions
- [ ] Define the `.ag` file extension and a first `examples/hello.ag`

## Milestone 1 — Lexer (Tokenizer)

- [ ] Define `Token` enum (keywords, identifiers, literals, operators, punctuation)
- [ ] Define `Span` (byte offsets + line/column) for precise diagnostics
- [ ] Skip whitespace and handle `//` and `/* */` comments
- [ ] Lex integer, float, string, boolean, and char literals
- [ ] Lex identifiers and keywords (`function`, `type`, `struct`, `enum`, `let`, `return`, …)
- [ ] Lex multi-char operators (`->`, `==`, `>=`, `?`, `|`, …)
- [ ] Emit diagnostic on invalid/unterminated tokens
- [ ] Unit tests: valid token streams + error cases

## Milestone 2 — AST & Parser

- [ ] Define AST node types (`Module`, `FunctionDecl`, `Param`, `TypeExpr`, `Stmt`, `Expr`, …)
- [ ] Implement recursive-descent parser skeleton
- [ ] Parse function signatures (`function name(params) -> Type effects(...)`)
- [ ] Parse function bodies and `return` statements
- [ ] Parse `let` bindings
- [ ] Parse expressions with correct precedence (binary, unary, calls, member access)
- [ ] Parse type expressions (named, generic like `Result<T, E>`, `Array<T>`)
- [ ] Parse `struct` and `enum` declarations
- [ ] Error recovery: report a diagnostic and recover to the next statement
- [ ] Snapshot tests: source → AST

## Milestone 3 — Static Type Checking

- [ ] Define type representation (`i32`, `f64`, `Bool`, `String`, `Array<T>`, `Map<K,V>`, user types)
- [ ] Build symbol table / scoped environment
- [ ] Resolve type names (including forward references within a module)
- [ ] Type-check `let` bindings (inference where unambiguous, no implicit conversions)
- [ ] Type-check expressions and operators
- [ ] Type-check function calls (arity + argument types + return type)
- [ ] Type-check `return` against declared return type
- [ ] Detect undefined symbols and duplicate declarations
- [ ] Unit tests: accept valid programs, reject invalid ones with correct error codes

## Milestone 4 — Structs, Enums & Functions

- [ ] Register struct definitions and their fields
- [ ] Register enum definitions and variants
- [ ] Type-check struct literal construction and field access
- [ ] Type-check enum variant construction
- [ ] (Stretch) basic `match` on enums
- [ ] Type-check function declarations end-to-end
- [ ] Integrate with symbol table and type resolution

## Milestone 5 — `Result` and `Option`

- [ ] Add `Option<T>` and `Result<T, E>` as built-in generic types
- [ ] Implement `Option`/`Result` construction (`Some`, `None`, `Ok`, `Err`)
- [ ] Type-check `.map(...)` style method calls (start with a minimal set)
- [ ] Implement `?` operator propagation with correct error-type checking
- [ ] Enforce explicit handling (no implicit unwrap)
- [ ] Unit tests for success and error propagation paths

## Milestone 6 — JSON Compiler Diagnostics

- [ ] Define `Diagnostic` struct: `code`, `severity`, `file`, `line`, `col`, `span`, `symbol`, `expected`, `received`, `message`, `repair`, `repairable`, `affects_other_files`
- [ ] Assign stable error codes (`TYPE_MISMATCH`, `UNDEFINED_SYMBOL`, `ARITY_MISMATCH`, `PARSE_ERROR`, …)
- [ ] Populate `expected`/`received` from type-checker failures
- [ ] Add `repair` suggestions (e.g., `replace_expression`, `add_import`)
- [ ] Serialize diagnostics to JSON (stable output schema)
- [ ] Human-readable fallback renderer for local dev
- [ ] Golden/snapshot tests for the JSON schema

## Milestone 7 — CLI & Integration

- [ ] Implement `agent check <file>` → parse + type-check → exit code + JSON diagnostics
- [ ] Implement `agent build` stub (compiles to a no-op / IR placeholder for now)
- [ ] Wire diagnostics to stdout as JSON with `--json` flag
- [ ] End-to-end test: broken `.ag` file → JSON → simulated fix → clean check
- [ ] Write an LLM-driven test harness (feed diagnostic JSON to a model, verify self-repair loop)

---

## Phase 1 Exit Criteria

- [ ] All above milestones complete
- [ ] A first-attempt-compile benchmark baseline recorded (see "Day one" metrics)
- [ ] `agent check` usable by an autonomous agent with zero human intervention
- [ ] Documented JSON diagnostic schema in `docs/diagnostics.md`

---

## Suggested Build Order

Do **vertical slices**, not horizontal layers. Get one thing working end-to-end before adding breadth:

1. **Skeleton:** `function main() -> i32 { return 0; }` parses and type-checks clean.
2. **One error path:** a type mismatch produces valid JSON diagnostics.
3. **Grow the grammar** (let, expressions, calls) behind the same diagnostic pipeline.
4. **Add structs/enums**, then `Result`/`Option`, then `?`.
5. **Harden** with the LLM self-repair test harness.

This keeps the differentiator (structured diagnostics) visible from day one instead of bolting it on at the end.
