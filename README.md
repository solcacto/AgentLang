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

## Getting Started

See [`Overview.md`](Overview.md) for the full design document, including the eight essential features, CLI commands (`agent check`, `agent build`, `agent test`, `agent audit`, `agent verify`, `agent impact`, `agent diff`, `agent patch`, `agent commit-check`), architecture, and proposed feature priorities.

See [`Overview.txt`](Overview.txt) for the original design text.

## Contributing

This is an early-stage design document. The four key architectural decisions are still open:

1. **Primary use case:** Agentic coding (leaning)
2. **How LLMs write code:** Hybrid human-readable + structured interface (leaning)
3. **Verification strictness:** Balanced (leaning)
4. **Compilation targets:** Native and WASM equally (leaning)

Open an issue to discuss any of these.

## License

Not yet specified.