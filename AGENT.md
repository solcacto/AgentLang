# Using AgentLang with OpenCode

AgentLang is a statically typed, capability-secured language whose compiler
speaks to agents. Instead of reading prose compiler output, your OpenCode agent
parses structured JSON diagnostics and applies fixes automatically.

## 1. Get the compiler

The `agent` binary is built from the AgentLang Rust project:

```bash
# From the AgentLang repo:
cargo build --release
cp target/release/agent /path/to/your/project/
```

Or download the latest release from
https://github.com/solcacto/AgentLang/releases.

Add it to your project directory (or PATH). Verify:

```bash
agent check --help
```

## 2. The agent loop

For every `.ag` file you want to verify:

```bash
agent check --format json program.ag
```

- Exit code `0` → stdout is `[]` (clean, no diagnostics).
- Exit code `1` → stdout is a JSON array of diagnostics.
- Exit code `2` → usage or I/O error.

**Loop:**

1. Agent writes or edits `program.ag`.
2. OpenCode runs `agent check --format json program.ag`.
3. If `[]`, done. Otherwise, the agent reads the JSON array and applies fixes.
4. Repeat until clean.

## 3. Diagnostic JSON schema

Every diagnostic is a JSON object:

| Field | Type | Meaning |
|-------|------|---------|
| `severity` | `"error"` | Always `"error"` in Phase 1 |
| `code` | string | Stable error code (see below) |
| `file` | string | Source file path |
| `line` | integer | 1-based line |
| `col` | integer | 1-based column |
| `symbol` | string? | Symbol responsible (variable, field, function) |
| `expected` | string? | Expected type/arity |
| `received` | string? | Actual type/arity |
| `message` | string | Human-readable explanation |
| `repair` | object? | Structured repair suggestion (`type`, `target`, `expected_type`) |
| `repairable` | boolean | Whether an automatic repair exists |
| `affects_other_files` | boolean | Whether other files may be affected |

### Stable error codes

| Code | Meaning | Typical fix |
|------|---------|-------------|
| `TYPE_MISMATCH` | Expected and actual types differ | Replace expression with one of `expected` type |
| `UNDEFINED_SYMBOL` | Variable not declared | `let <symbol> = <init>;` before first use |
| `UNDEFINED_FUNCTION` | Call to unknown function | Define the function or add import |
| `ARITY_MISMATCH` | Wrong number of arguments | Add/remove arguments |
| `DUPLICATE_SYMBOL` | Symbol declared twice | Remove the duplicate declaration |
| `MISSING_RETURN` | Non-unit function doesn't return on every path | Add `return <value>;` before closing `}` |
| `UNKNOWN_FIELD` | Field doesn't exist on struct | Use an existing field name |
| `MISSING_FIELD` | Struct literal missing required field | Add `<field>: <value>` to the literal |
| `NOT_A_STRUCT` | Field access on non-struct type | Access a field on a struct value |
| `UNKNOWN_VARIANT` | Enum variant doesn't exist | Use a valid variant |
| `IMMUTABLE_ASSIGNMENT` | Assignment to non-`let mut` binding | `let mut <name> = ...;` |
| `NOT_INDEXABLE` | Indexing a non-collection | Use an `Array<T>` or `Map<K, V>` |
| `NOT_ITERABLE` | `for` over non-collection | Iterate over `Array<T>` or `Map<K, V>` |
| `EFFECT_VIOLATION` | Calling function with undeclared effect | Add effect to caller's `effects(...)` clause |
| `PARSE_ERROR` | Unexpected token or missing syntax | Fix syntax (e.g. missing `;`) |

### Repair object

| Field | Type | Meaning |
|-------|------|---------|
| `type` | string | Repair category (`replace_expression`, `add_import`, `declare_symbol`, `insert_return`, `add_field`, `add_mut`, `add_effect`, `rename_symbol`) |
| `target` | string? | Symbol/expression to modify |
| `expected_type` | string? | Type the repair should produce |

## 4. Drop-in OpenCode repairer

Copy `tools/opencode_repairer.py` into your project directory. It reads
`AGENTLANG_SOURCE_FILE` and `AGENTLANG_DIAGNOSTICS_FILE` (set by
`CommandRepairer`) and applies deterministic fixes for every error code.

### Benchmark your agent's repair ability

```bash
# Run the full 23-task benchmark with the built-in rule-based repairer
cargo run -- bench benchmarks \
    --repairer command \
    --repair-cmd "python tools/opencode_repairer.py" \
    --max-iters 5

# Expected: 23 / 23 resolved, mean iterations ~1.05
```

### Use OpenCode itself as the repairer

Create `tools/opencode_repairer.py` that calls OpenCode with a fix-it prompt:

```python
#!/usr/bin/env python3
import json, os, subprocess, sys

source_file = os.environ["AGENTLANG_SOURCE_FILE"]
diag_file = os.environ["AGENTLANG_DIAGNOSTICS_FILE"]

with open(source_file) as f:
    source = f.read()
with open(diag_file) as f:
    diags = json.load(f)

prompt = f"""Fix this AgentLang source so it passes `agent check`.

Diagnostics:
{json.dumps(diags, indent=2)}

Current source:
{source}

Output only the repaired source code, no explanation."""

result = subprocess.run(
    ["opencode", "prompt", prompt],
    capture_output=True, text=True,
)

if result.returncode == 0:
    with open(source_file, "w") as f:
        f.write(result.stdout.strip())
sys.exit(0)
```

Then:

```bash
cargo run -- bench benchmarks \
    --repairer command \
    --repair-cmd "python tools/opencode_repairer.py" \
    --max-iters 5
```

AgentLang holds **no API keys** — your repair command brings its own model and credentials.

## 5. Language subset (Phase 1)

```ag
// Functions with typed params and effects
function add(a: i32, b: i32) -> i32 effects(pure) {
    return a + b;
}

// Structs and enums
struct Point {
    x: i32,
    y: i32,
}

enum Color {
    Red,
    Green,
    Blue,
}

// let / let mut, assignment, return
let mut total = 0;
total = total + 1;
return total;

// if / else if / else with block scoping
if x > 0 {
    return 1;
} else if x < 0 {
    return -1;
} else {
    return 0;
}

// while loops
while i < limit {
    total = total + i;
    i = i + 1;
}

// for loops over collections
for x in items {
    total = total + x;
}

// Array and Map indexing
let first = items[0];
let score = scores[name];

// Field and index mutation (root binding must be let mut)
let mut p = Point { x: 1, y: 2 };
p.x = 10;
items[0] = 5;

// match with exhaustiveness
match c {
    Color::Red => 1,
    _ => 0,
}

// Option / Result with ?
let value = parse(input)?;
```

## 6. Example session

```bash
# 1. Write a broken program
cat > program.ag <<'EOF'
function main() -> i32 {
    return "not a number";
}
EOF

# 2. Check it (JSON diagnostics)
agent check --format json program.ag
# → [{"code":"TYPE_MISMATCH","expected":"i32","received":"String",...}]

# 3. Agent fixes it based on the JSON
cat > program.ag <<'EOF'
function main() -> i32 {
    return 0;
}
EOF

# 4. Re-check — clean
agent check --format json program.ag
# → []
```

## 7. Files to copy into a new project

| File | Purpose |
|------|---------|
| `agent` (or `agent.exe`) | The AgentLang compiler binary |
| `tools/opencode_repairer.py` | Drop-in repair script for `CommandRepairer` |
| `benchmarks/` | 23 example tasks (broken + fixed pairs) |
| `examples/` | Example `.ag` programs to copy from |

## 8. Quick start checklist

- [ ] Copy `agent` binary into project
- [ ] Copy `tools/opencode_repairer.py` into project
- [ ] Write `program.ag`
- [ ] Run `agent check --format json program.ag`
- [ ] Agent reads JSON, edits `program.ag`
- [ ] Repeat until `[]`

That's it — no API keys in the compiler, no complex setup, just a binary and a JSON contract.