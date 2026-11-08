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
```

Output is JSON. The command exits non-zero if any task is left unresolved.

```json
{
  "total": 6,
  "first_pass_ok": 1,
  "fixed": 6,
  "unresolved": 0,
  "avg_iterations": 1.0,
  "total_diagnostics": 5
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

A real LLM adapter lives outside the core crate: serialize `source` and `diags`
into a prompt, call the model, and return the rewritten program. The harness only
cares that the returned source makes progress, so adapters can be swapped freely.

## Adding a task

Create a directory and a broken program, then a known-good version:

```text
benchmarks/07_unknown_field/
├── broken.ag
└── fixed.ag
```

Keep each task focused on roughly one diagnostic code so that the first
diagnostic (`first_code` in the report) is a stable signal. Run
`cargo test` — `benchmark_suite_resolves_with_oracle` loads the whole directory
and fails if the oracle cannot resolve every task.

## Baseline

Recorded 2026-11-09 on the seven-task corpus (`01_ok_baseline` … `07_non_exhaustive_match`):

| Metric | Value |
| --- | --- |
| First-pass clean | 1 / 7 |
| Resolved with oracle | 7 / 7 |
| Mean iterations (non-clean) | 1.0 |
| Unresolved | 0 |

The oracle number is an upper bound, not a model result. The meaningful metric
will be produced once a concrete LLM `Repairer` is wired in; the corpus and
report schema are already in place to measure it.
