# AgentLang Diagnostics Schema

Every compiler diagnostic is emitted as structured JSON. This is a stable,
machine-readable contract intended for autonomous agents — not a
human-readable log format.

## Shape

```json
{
  "severity": "error",
  "code": "TYPE_MISMATCH",
  "file": "src/player.ag",
  "line": 42,
  "col": 12,
  "symbol": "player.health",
  "expected": "i32",
  "received": "String",
  "message": "Return type mismatch: expected `i32`, found `String`",
  "repair": {
    "type": "replace_expression",
    "target": "player.health",
    "expected_type": "i32"
  },
  "repairable": true,
  "affects_other_files": false
}
```

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `severity` | `"error" \| "warning"` | Diagnostic level |
| `code` | string | Stable error identifier (see below) |
| `file` | string | Path of the file |
| `line` | integer | 1-based line number |
| `col` | integer | 1-based column number |
| `symbol` | string? | Symbol responsible, when applicable |
| `expected` | string? | Expected type/arity, when applicable |
| `received` | string? | Actual type/arity, when applicable |
| `message` | string | Human-readable explanation |
| `repair` | object? | Suggested structured repair |
| `repairable` | boolean | Whether an automatic repair exists |
| `affects_other_files` | boolean | Whether other files may be affected |

Optional fields are omitted when absent.

## Machine mode

By default `agent check` prints a human-readable `OK:` line on success. Agent
harnesses should use machine mode, which always writes a JSON array to stdout and
nothing else there:

```bash
agent check --format json path/to/file.ag
agent check --json path/to/file.ag        # alias
```

- Exit code `0` — stdout is `[]`.
- Exit code `1` — stdout is an array with one or more diagnostics.
- Exit code `2` — usage or I/O error (stdout is not guaranteed to be JSON).

This lets an agent call the compiler as a tool without guessing at the output
format or parsing prose.

## Repair object

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Repair category (`replace_expression`, `add_import`, `declare_symbol`, `insert_return`, `add_field`, `add_mut`, `rename_symbol`) |
| `target` | string? | Symbol/expression to modify |
| `expected_type` | string? | Type the repair should produce |

## Error codes (Phase 1)

| Code | Meaning |
|------|---------|
| `LEX_ERROR` | Invalid character, unterminated string/comment |
| `PARSE_ERROR` | Unexpected token or missing syntax |
| `TYPE_MISMATCH` | Expected and actual types differ |
| `UNDEFINED_SYMBOL` | Variable not declared |
| `UNDEFINED_FUNCTION` | Call to an unknown function |
| `ARITY_MISMATCH` | Wrong number of arguments |
| `DUPLICATE_SYMBOL` | Symbol/type/variant declared twice |
| `MISSING_RETURN` | Non-unit function does not return on every path |
| `UNKNOWN_TYPE` | Struct literal names an undefined struct |
| `UNKNOWN_FIELD` | Field does not exist on the struct |
| `MISSING_FIELD` | Struct literal is missing a required field |
| `DUPLICATE_FIELD` | Field declared or initialized twice |
| `NOT_A_STRUCT` | Field access on a non-struct type |
| `UNKNOWN_VARIANT` | Enum/variant does not exist |
| `IMMUTABLE_ASSIGNMENT` | Assignment to a binding not declared with `let mut` (covers variables, struct fields, and index targets) |
| `NOT_INDEXABLE` | Indexing a type that is not an `Array` or `Map` |
| `NOT_ITERABLE` | `for` loop over a type that is not an `Array` or `Map` |
| `UNKNOWN_METHOD` | Method does not exist on the receiver type |
| `INVALID_TRY` | `?` used outside a matching `Option`/`Result` context |
| `TRY_TYPE_MISMATCH` | `?` propagates a different error type than the function returns |
| `EXPECTED_LAMBDA` | `.map` was given a non-closure argument |
| `UNSUPPORTED_LAMBDA` | Closure used outside a supported position |
| `NOT_AN_ENUM` | `match` uses variant patterns on a non-enum value |
| `NON_EXHAUSTIVE_MATCH` | `match` does not cover every variant |
| `DUPLICATE_MATCH_ARM` | Two arms match the same enum variant |

## Contract for agents

1. Parse the JSON array.
2. For each `repairable` diagnostic, apply `repair` (or reason from
   `expected`/`received`).
3. Re-run `agent check`.
4. Stop when the array is empty.
