# AgentLang — Project Overview

**AgentLang** is a proposed programming language designed around a simple premise: the primary programmer is an advanced LLM (autonomous coding agent), not a human. Most existing languages were designed around human limitations (typo-prone syntax, need for readable abstractions, slow at repetition, etc.). AgentLang assumes code will be generated, modified, tested, and maintained by autonomous agents.

---

## TL;DR

| Aspect | Summary |
|--------|---------|
| **Core idea** | A statically typed, memory-safe, compiled language optimized for machine-generated software |
| **Key differentiator** | The compiler actively participates in the agent's reasoning process — not just a syntax checker |
| **Security model** | Capability-based permissions; memory-safe by default |
| **Verification** | Continuous/incremental compilation with structured, machine-readable diagnostics |
| **Target platforms** | Native (LLVM), WebAssembly, eventually embedded |
| **Recommended roadmap** | Build a small, fully functional compiler + toolchain first (5 phases) |

---

## Open Questions / Decisions Needed

These four architectural choices will significantly influence the design. They should be decided early and revisited as the project progresses.

| Decision | Options | Current leaning | Status |
|----------|---------|-----------------|--------|
| **Primary use case** | General-purpose programming / Agentic coding / Both equally | Agentic coding | Open |
| **How LLMs write code** | Traditional human-readable syntax only / Structured AST/JSON generation primarily / Hybrid (source + structured interface) | Hybrid | Open |
| **Verification strictness** | Balanced / Extremely strict / Fast iteration | Balanced | Open |
| **Compilation targets** | Native executables first / WebAssembly first / Native and WASM equally | Native and WASM equally | Open |

Use these design preferences to guide prioritization during development.

---

## 1. Fundamental Concept

> A programming language built for machine-generated software.

**AgentLang** is a statically typed, memory-safe, compiled language with:
- Explicit permissions
- Machine-readable diagnostics
- Incremental verification
- Deterministic behaviour

**Three pillars:** Security first · Continuous verification · Performance · Agent-native tooling

### Design inspirations

| Language | What to borrow |
|----------|---------------|
| Rust | Ownership, borrowing, memory safety |
| Go | Simplicity, fast compilation, concurrency |
| Zig | Explicit behaviour, predictable execution |
| TypeScript | Accessible type system and developer experience |
| Erlang | Fault isolation and concurrency |
| Haskell | Pure functions and functional safety |
| WebAssembly | Portable sandboxed execution |

The language would not simply combine these — it would change the underlying *development model*.

---

## 2. Design Principle: Optimise for LLM Generation

Make the language **deliberately restrictive**:

- One obvious way to perform a particular operation
- No implicit type conversions
- No hidden side effects
- No ambiguous operator behaviour
- No implicit dependencies
- No global mutable state by default
- No complicated inheritance systems
- Explicit function signatures
- Explicit error handling
- Explicit permissions

**Why?** An LLM doesn't benefit from ten ways of doing the same thing. Reducing the number of valid approaches makes generated code more predictable.

### Example contrast

**Typical flexible language:**
```python
def get_user(id):
    user = database.find(id)
    if user:
        return user["name"]
    return None
```

Ambiguities: type of `id`, what `find` returns, whether the database can fail, whether `user["name"]` exists, what permissions the function has.

**Proposed AgentLang:**
```agentlang
function get_user(
    id: UserId
) -> Result<Option<UserName>, DatabaseError>
    effects(database.read)
{
    let user = database.find(id)?;
    return user.map(|u| u.name);
}
```

The second version communicates substantially more information to both the compiler and the LLM. The compiler can validate behaviour without executing it.

---

## 3. Essential Features (Eight Major Systems)

### A. Strict but approachable type system

- Static typing everywhere: every variable, function, parameter, and return value has a known type
- Core types: `i8/i32/i64`, `f32/f64`, `Boolean`, `String`, `Array`, `Map`, `Struct`, `Enum`, `Option`, `Result`, Custom types
- **Refinement types** supported:
  ```agentlang
  type UserAge = Integer where value >= 0 and value <= 130;
  type Percentage = Float where value >= 0.0 and value <= 100.0;
  type Email = String where valid_email(value);
  ```

The compiler catches invalid values before execution.

### B. Explicit security permissions (capability system)

Security is part of the language itself, not left to libraries or runtime configuration. Every function declares what it can access:

```agentlang
function read_config()
    -> Result<Config, IOError>
    effects(filesystem.read("/config"))
{
    return filesystem.read("/config");
}

function calculate_total(
    prices: Array<Float>
) -> Float
    effects(pure)
{
    return prices.sum();
}
```

A function that doesn't declare filesystem access cannot access the filesystem.

| Capability | Permission |
|------------|-----------|
| `filesystem.read` | Read permitted files |
| `filesystem.write` | Write permitted files |
| `network.connect` | Connect to permitted hosts |
| `database.read` | Read database |
| `database.write` | Modify database |
| `process.spawn` | Launch processes |
| `environment.read` | Read permitted environment variables |

Permissions are granted explicitly by the host environment — an agent-generated program should never automatically receive full access simply because it compiles.

### C. Compiler-native error detection

Traditional compiler errors are designed for humans. AgentLang treats compiler diagnostics as a **first-class machine interface**.

Instead of:
```
Error: mismatched types
Expected Integer, found String
```

The compiler returns structured JSON:
```json
{
  "status": "error",
  "code": "TYPE_MISMATCH",
  "file": "src/player.ag",
  "line": 42,
  "symbol": "player.health",
  "expected": "Integer",
  "received": "String",
  "repair": {
    "type": "replace_expression",
    "target": "player.health",
    "expected_type": "Integer"
  }
}
```

Every diagnostic includes:
- Unique error identifier
- File and location
- Exact symbol or expression responsible
- Expected and actual types
- Explanation of the violation
- Suggested repair category
- Whether the error can be automatically repaired
- Whether other files are affected

This avoids dumping thousands of lines of compiler output into an LLM's context window.

### D. Continuous compilation and verification

**Live Verification Engine:** Instead of recompiling the entire project after every change, the compiler maintains an incremental representation of the project.

**Agent development loop:**
1. Agent generates code (new function, module, or modification)
2. Incremental compiler analyses only changed code and affected dependencies
3. Static verification: types, memory safety, permissions, contracts, dependencies
4. Automated testing: relevant unit and integration tests execute
5. Agent receives structured feedback (only relevant errors and failed assertions)
6. Commit validation: change checked against project verification requirements

Commit permitted only when required checks pass.

### E. Built-in contracts and assertions

Functions optionally declare expected behaviour:

```agentlang
function withdraw(
    account: Account,
    amount: Money
) -> Result<Account, BankingError>

requires amount > Money.zero()
requires account.balance >= amount

ensures result.balance == account.balance - amount
{
    return account.with_balance(
        account.balance - amount
    );
}
```

The compiler statically proves what it can. For complex conditions, an SMT solver attempts to prove them. If unprovable, the contract is reported as unverified — not assumed safe.

Contracts optional for ordinary functions, mandatory for security-critical operations (financial calculations, authentication, permissions, data transformations, game logic, inventory systems, network protocols).

### F. Memory safety without Rust-level complexity

- Automatic memory management by default
- Deterministic resource cleanup
- Ownership rules
- Borrow checking where necessary
- No unrestricted pointer arithmetic
- No use-after-free
- No data races

Agents don't manually manage memory in ordinary applications. For performance-critical applications, an explicitly marked low-level mode with tighter rules is provided.

### G. Native concurrency and parallel execution

Concurrency is a first-class language feature:

```agentlang
function load_dashboard()
    -> Result<Dashboard, Error>
    effects(network.read)
{
    let (profile, messages, statistics) = parallel {
        get_profile(),
        get_messages(),
        get_statistics()
    };

    return Dashboard {
        profile: profile?,
        messages: messages?,
        statistics: statistics?
    };
}
```

Features:
- Structured concurrency
- Automatic cancellation
- Race detection
- Deadlock prevention where statically possible
- Explicit shared-state permissions
- Automatic parallelisation of independent, pure operations

### H. Structured code representation

Agents shouldn't have to work only with raw text. Provide two interfaces:
1. A human-readable source language
2. A structured machine-readable representation

The agent can create/modify functions directly through structured operations:

```json
{
  "operation": "create_function",
  "module": "player",
  "name": "calculate_health",
  "parameters": [
    { "name": "damage", "type": "Integer" }
  ],
  "return_type": "Integer",
  "body": {
    "operation": "subtract",
    "left": { "symbol": "player.health" },
    "right": { "symbol": "damage" }
  }
}
```

The compiler validates structure before turning it into executable code. The agent doesn't worry about missing brackets, punctuation, formatting, indentation, or inconsistent syntax — and can modify an individual function without rewriting the entire file.

First-class compiler API operations:
- `create_function`
- `modify_function`
- `replace_expression`
- `add_import`
- `modify_type`
- `remove_symbol`
- `rename_symbol`

Every modification is checked against the current project version.

---

## 4. The Development Environment as Part of the Language

Don't treat the compiler, package manager, testing framework, and agent interface as separate products — develop them together.

**Proposed CLI:**

| Command | Purpose |
|---------|---------|
| `agent check` | Fast static verification |
| `agent build` | Compile project |
| `agent test` | Run tests |
| `agent audit` | Security audit |
| `agent verify` | Run contracts and deeper verification |
| `agent impact` | Show consequences of a proposed change |
| `agent diff` | Show structural changes |
| `agent patch` | Apply a validated structural modification |
| `agent commit-check` | Run all required pre-commit checks |

**Example — `agent impact src/player.ag`:**
```
CHANGE IMPACT ANALYSIS

Modified:
  Player.calculate_health()

Direct dependencies:
  Player.take_damage()
  Combat.resolve_attack()

Indirect dependencies:
  MatchController.process_hit()

Potential issues:
  1 Type mismatch
  1 Contract violation
  0 Security violations

Unmodified modules affected: 2

Verification: FAILED
```

---

## 5. Architecture and Compilation

Build around an **intermediate representation (IR)** rather than inventing a new execution environment:

```
LLM / Coding Agent
(Text generation or structured AST operations)

AgentLang Compiler
(Parsing, types, effects, contracts, security)

Verified Intermediate Representation
(Optimised, typed, dependency-tracked program)

Native        → LLVM / machine code
Web           → WebAssembly
Sandbox       → Isolated execution
```

**Initial target platforms:** Linux, Windows, macOS, WebAssembly (embedded eventually).

Use LLVM for the native backend rather than building architecture-specific compilers from scratch. WASM support uses a restricted execution profile with explicitly granted host capabilities.

---

## 6. Existing AI-Focused Languages to Study

| Project | Focus |
|---------|-------|
| **Promise** | Explicit ownership, effects and error handling; designed around AI-generated maintainable code |
| **Edict** | Structured JSON AST, effect tracking, contract verification, WASM sandboxing |
| **Orison** | JSON diagnostics, structural patches, capability-secured effects, agent-focused tooling |
| **QUASAR** | Automatic parallelisation, uncertainty quantification, secure agent-generated code |
| **Lacuna** | (Research language) |

Related research: a 2025 PLDI paper demonstrated that constraining LLM generation using type information reduced compilation errors by more than half.

**Conclusion:** Don't just make another language with better syntax — build a language where the compiler actively participates in the agent's reasoning and development process.

---

## 7. Proposed Feature Priorities

| Feature | Priority | Reason |
|---------|----------|--------|
| Static type system | Critical | Prevent invalid code |
| Structured diagnostics | Critical | Enable autonomous repairs |
| Memory safety | Critical | Eliminate major vulnerability classes |
| Capability security | Critical | Restrict agent permissions |
| Incremental compilation | Critical | Fast development feedback |
| Structural code editing | Critical | Avoid destructive rewrites |
| Built-in testing | High | Catch logical errors |
| Contract verification | High | Verify intended behaviour |
| Native compilation | High | Performance |
| WASM support | High | Portability and isolation |
| Automatic parallelisation | Medium | Performance optimisation |
| Formal verification | Medium | Stronger correctness guarantees |

---

## 8. Proposed Development Roadmap

Build a small but fully functional compiler and toolchain first, not the entire language immediately.

**Phase 1 — Language core**
- [ ] Lexer and parser
- [ ] Basic syntax
- [ ] Static type checking
- [ ] Structs, enums and functions
- [ ] `Result` and `Option` types
- [ ] JSON compiler diagnostics

**Phase 2 — Agent toolchain**
- [ ] Structured AST interface
- [ ] Function-level code modification
- [ ] Incremental compilation
- [ ] Dependency tracking
- [ ] Machine-readable errors
- [ ] Automated repair feedback

**Phase 3 — Security**
- [ ] Capability-based permissions
- [ ] Memory safety
- [ ] Effect tracking
- [ ] Sandboxed execution
- [ ] Security auditing

**Phase 4 — Production compiler**
- [ ] LLVM backend
- [ ] Optimisation
- [ ] Native executables
- [ ] WASM backend
- [ ] Standard library
- [ ] Package manager

**Phase 5 — Verification ecosystem**
- [ ] Contract system
- [ ] SMT integration
- [ ] Automatic test generation
- [ ] Change impact analysis
- [ ] Pre-commit verification
- [ ] Agent development protocol

**Day one:** Create a benchmark suite specifically for LLM-generated software. Measure:
- [ ] How often different LLMs generate compilable code on the first attempt
- [ ] How many iterations are required to fix compiler errors
- [ ] How often generated code introduces security vulnerabilities
- [ ] How often a modification breaks unrelated functionality
- [ ] How quickly the compiler identifies errors
- [ ] How much context the agent needs to understand a project
- [ ] How much time and memory compiled applications consume
- [ ] How easily different LLM families can generate code in the language

Objective: optimise the **entire agentic development lifecycle**, not simply the runtime.

---

## 9. Three Decisions to Make Before Starting

1. **Primary use case:** General-purpose programming / Agentic coding / Both equally
2. **How LLMs write code:** Traditional human-readable syntax only / Structured AST/JSON generation primarily / Hybrid (human-readable + structured agent interface)
3. **Verification strictness:** Balanced / Extremely strict / Fast iteration
4. **Compilation targets:** Native first / WASM first / Native and WASM equally

**Personal recommendation:** Hybrid human-readable/structured interface, strict static typing, capability-based security, incremental verification, and both native and WASM compilation.

---

## Bottom Line

The most important differentiator wouldn't be that an LLM can write the language easily. It would be that an LLM can **write it, validate it, repair it, modify it, and safely commit it with minimal human intervention.**

---

*Converted from `Overview.txt` — see the original text file for full detail.*