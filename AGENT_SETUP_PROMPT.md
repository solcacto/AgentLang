# AgentLang OpenCode Setup Prompt

Copy and paste this entire block into your OpenCode chat to set up AgentLang in your project:

---

You are helping me set up AgentLang in this project. Please do the following:

1. **Create project structure**:
   - Create a folder named `agentlang/` at the project root
   - Inside it, create `bin/`, `tools/`, `examples/`, `benchmarks/` subfolders

2. **Copy AgentLang files from the repo** at `https://github.com/solcacto/AgentLang.git`:
   - Copy `tools/opencode_repairer.py` → `agentlang/tools/`
   - Copy `AGENT.md` → `agentlang/AGENT.md`
   - Copy `benchmarks/` (all task directories) → `agentlang/benchmarks/`
   - Copy `examples/` → `agentlang/examples/`

3. **Build the `agent` binary**:
   - Clone the AgentLang repo to a temp location
   - Run `cargo build --release`
   - Copy `target/release/agent` (or `agent.exe` on Windows) → `agentlang/bin/`

4. **Create a test program**:
   - Create `test.ag` at project root with:
     ```ag
     function main() -> i32 {
         return 42;
     }
     ```

5. **Verify it works**:
   - Run: `agentlang/bin/agent check --format json test.ag` → should output `[]`
   - Run: `agentlang/bin/agent run test.ag` → should exit with code 42

6. **Confirm everything works** and report back which files were copied and where.

The full usage documentation is in `agentlang/AGENT.md` — read it for the complete language reference, diagnostic schema, and benchmark harness instructions.

---

## What this prompt does

| Step | What happens |
|------|-------------|
| 1-2 | Downloads the compiler, repair script, benchmarks, examples, and docs |
| 3 | Compiles the `agent` binary from source |
| 4-5 | Creates and verifies a test program |
| 6 | You're ready to write AgentLang programs |

## After setup

Your project structure looks like:

```
my-project/
├── agentlang/
│   ├── bin/agent (or agent.exe)
│   ├── tools/opencode_repairer.py
│   ├── AGENT.md
│   ├── benchmarks/ (23 tasks)
│   └── examples/ (sample programs)
├── test.ag
└── your-program.ag
```

Then just run:
```bash
agentlang/bin/agent check --format json your-program.ag
agentlang/bin/agent run your-program.ag
```
