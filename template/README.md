# AgentLang Starter Template

Copy this folder into your new project to get started with AgentLang + OpenCode.

## Project Structure

```
my-agentlang-project/
├── agent                    # (or agent.exe) - compiled compiler binary
├── tools/
│   └── opencode_repairer.py # Deterministic repair agent for benchmarks
├── examples/
│   └── hello.ag             # Example program
├── benchmarks/              # Self-repair benchmark tasks
├── AGENT.md                 # Full usage instructions
└── program.ag               # Your first program
```

## Quick Start

1. **Build the compiler** (once):
   ```bash
   cargo build --release --manifest-path path/to/agentlang/repo/Cargo.toml
   cp path/to/agentlang/repo/target/release/agent .
   ```

2. **Write your first program** (`program.ag`):
   ```ag
   function main() -> i32 {
       return 42;
   }
   ```

3. **Check and run**:
   ```bash
   agent check --format json program.ag
   agent run program.ag
   ```

4. **Run the benchmark suite**:
   ```bash
   cargo run --manifest-path path/to/agentlang/repo/Cargo.toml -- bench benchmarks \
       --repairer command --repair-cmd "python tools/opencode_repairer.py" --max-iters 5
   ```

## Files to Copy from AgentLang Repo

| Source | Destination | Purpose |
|--------|------------|---------|
| `target/release/agent` | `./agent` | Compiler binary (build once) |
| `tools/opencode_repairer.py` | `./tools/` | Repair agent for benchmarks |
| `benchmarks/` | `./benchmarks/` | 23 test tasks |
| `AGENT.md` | `./AGENT.md` | Full instructions |
| `examples/` | `./examples/` | Sample programs |
