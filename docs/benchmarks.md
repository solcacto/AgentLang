# Self-Repair Benchmarks

The central claim of AgentLang is that an autonomous agent can read the
compiler's structured diagnostics, repair the program, and iterate to a clean
check **without a human reading compiler output**. This harness turns that claim
into a number.

## How a task works

A task is a directory under `benchmarks/` containing:

| File | Required | Meaning |
| --- | --- | --- |
| `broken.ag` | yes | A program that fails `agent check` |
| `fixed.ag` | no | A known-good version (defaults to `broken.ag` if absent) |

The harness:

1. Runs `check_source` on `broken.ag` and records the diagnostics.
2. If clean, records `first_pass_ok = true` and stops.
3. Otherwise asks the `Repairer` for a rewritten program.
4. Re-checks the result and repeats until clean or `--max-iters` is reached.

## Running

```bash
# Oracle repairer (default): returns fixed.ag, validating the loop.
cargo run -- bench benchmarks

# First-pass only: never repairs, measures the broken corpus.
cargo run -- bench benchmarks --repairer none

# Custom directory and iteration budget.
cargo run -- bench path/to/tasks --max-iters 10

# Drive repairs through an external agent harness or CLI.
cargo run -- bench benchmarks \
  --repairer command \
  --repair-cmd "my-agent --file {source}"
```

Output is JSON. The command exits non-zero if any task is left unresolved.

```json
{
  "metadata": {
    "tool": "agentlang",
    "version": "0.1.0",
    "repairer": "oracle",
    "max_iters": 5
  },
  "total": 23,
  "first_pass_ok": 1,
  "fixed": 23,
  "unresolved": 0,
  "avg_iterations": 1.0,
  "total_diagnostics": 23
}
```

## The `Repairer` trait

`src/bench.rs` defines the extension point:

```rust
pub trait Repairer {
    fn repair(&mut self, task: &Task, source: &str, diags: &[Diagnostic]) -> Option<String>;
}
```

Return `Some(new_source)` to continue the loop, or `None` to give up. Built-in
implementations:

- `OracleRepairer` — returns `task.fixed`, proving every task is repairable and
  establishing an upper bound.
- `NoopRepairer` — always returns `None`, isolating first-pass behavior.
- `CommandRepairer` — runs an external command and reads back the edited file
  (see below).

A repairer only needs to make progress; the harness re-checks the result. That
makes provider and harness choice irrelevant to the compiler.

## Driving repairs with `CommandRepairer`

This is the recommended integration for agent harnesses (OpenCode, Claude Code,
custom scripts, local models). AgentLang does **not** hold credentials — the
command you pass owns its model and API key.

```bash
cargo run -- bench benchmarks \
  --repairer command \
  --repair-cmd "my-agent --file {source}"
```

For each repair round the harness creates a scratch directory containing:

| File | Meaning |
| --- | --- |
| `source.ag` | The current (broken) program. Rewrite it in place to repair. |
| `diagnostics.json` | The current diagnostics as pretty-printed JSON |

The command is run through the platform shell (`cmd /C` on Windows, `sh -c`
elsewhere). The result is whatever `source.ag` contains afterwards. If the file
is unchanged, the spawn fails, or the command exits non-zero, the task stops.

### Placeholders

| Placeholder | Value |
| --- | --- |
| `{dir}` | Scratch directory |
| `{source}` | Path to `source.ag` |
| `{diagnostics}` | Path to `diagnostics.json` |
| `{task}` | Task name |

The same values are exported as environment variables, which avoids quoting
problems when paths contain spaces:

| Variable | Value |
| --- | --- |
| `AGENTLANG_DIR` | Scratch directory |
| `AGENTLANG_SOURCE_FILE` | Path to `source.ag` |
| `AGENTLANG_DIAGNOSTICS_FILE` | Path to `diagnostics.json` |
| `AGENTLANG_TASK` | Task name |

A minimal repairer script (POSIX shell) that defers to another binary:

```sh
#!/bin/sh
my-llm-fix --json "$AGENTLANG_DIAGNOSTICS_FILE" --out "$AGENTLANG_SOURCE_FILE"
```

### Reproducibility

Reports record a `metadata` block with the tool, version, repairer, and
`max_iters`. When comparing models, also note the model ID and prompt/command
version outside the report — the same corpus with different models is not a
like-for-like comparison.

## Adding a task

Create a directory and a broken program, then a known-good version:

```text
benchmarks/08_unknown_field/
├── broken.ag
└── fixed.ag
```

Keep each task focused on roughly one diagnostic code so that the first
diagnostic (`first_code` in the report) is a stable signal. Run
`cargo test` — `benchmark_suite_resolves_with_oracle` loads the whole directory
and fails if the oracle cannot resolve every task.

## Baseline

Recorded 2026-11-09 on the twenty-three-task corpus (`01_ok_baseline` … `23_effect_subset_violation`):

| Metric | Value |
| --- | --- |
| First-pass clean | 1 / 23 |
| Resolved with oracle | 23 / 23 |
| Mean iterations (non-clean) | 1.0 |
| Unresolved | 0 |

### Model-driven repairer results

The same corpus was run with `--repairer command` using a rule-based repair
agent (`tools/repair_agent.py`) that reads `diagnostics.json` and applies
deterministic fixes. This simulates what a real LLM-based repairer would do:

| Metric | Value |
| --- | --- |
| Repairer | command (`python tools/repair_agent.py`) |
| Max iterations | 5 |
| First-pass clean | 1 / 23 |
| Resolved | 23 / 23 |
| Mean iterations (non-clean) | 1.05 |
| Unresolved | 0 |
| Total diagnostics seen | 24 |

All 23 tasks were resolved. Tasks needing more than one iteration were those
where an initial fix created a new diagnostic (e.g. changing a function's
return type to `Result<T, E>` then requires wrapping the return value in
`Result::Ok(...)`).

The oracle number is an upper bound, not a model result. To run with a real
LLM-based repairer, replace `python tools/repair_agent.py` with a command that
sends `diagnostics.json` to a model and writes the repaired program back to
`source.ag` — see "Testing with OpenCode" below.

## Testing with OpenCode (or any LLM-based repairer)

AgentLang holds no API keys — the repair command you provide owns its model
and credentials. To wire OpenCode as a repairer, create a script that:

1. Reads `diagnostics.json` (path from `AGENTLANG_DIAGNOSTICS_FILE`)
2. Reads `source.ag` (path from `AGENTLANG_SOURCE_FILE`)
3. Sends both to OpenCode with a prompt like:
   > "Here is an AgentLang source file and its compiler diagnostics. Fix the
   > source so it passes `agent check`. Output only the repaired source code."
4. Writes the repaired source back to `source.ag`
5. Exits 0 (the harness treats unchanged source as "give up")

### Example repair script for OpenCode

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
Source file: {source_file}
Diagnostics: {json.dumps(diags, indent=2)}

Current source:
{source}

Output only the repaired source code, no explanation."""

result = subprocess.run(
    ["opencode", "prompt", prompt],  # hypothetical OpenCode CLI
    capture_output=True, text=True
)

if result.returncode == 0:
    with open(source_file, "w") as f:
        f.write(result.stdout.strip())
sys.exit(0)
```

Then run:

```bash
cargo run -- bench benchmarks \
  --repairer command \
  --repair-cmd "python tools/opencode_repairer.py" \
  --max-iters 5
```

The `CommandRepairer` creates a scratch directory with `source.ag` and
`diagnostics.json`, runs your command (via `cmd /C` on Windows, `sh -c`
elsewhere), and reads back whatever is in `source.ag` afterward. Environment
variables `AGENTLANG_DIR`, `AGENTLANG_SOURCE_FILE`, `AGENTLANG_DIAGNOSTICS_FILE`,
and `AGENTLANG_TASK` are also exported for each round.

If your repairer needs multiple rounds (e.g. the first fix creates a new
diagnostic), the harness automatically re-checks and re-invokes the command up
to `--max-iters` times.
