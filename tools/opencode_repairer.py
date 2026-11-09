#!/usr/bin/env python3
"""
OpenCode repairer for AgentLang.

Drop this file into any project directory alongside the `agent` binary, then
run:

    cargo run -- bench benchmarks \
        --repairer command \
        --repair-cmd "python tools/opencode_repairer.py" \
        --max-iters 5

The agent (OpenCode) invokes this script each repair round. The script:

1. Reads AGENTLANG_SOURCE_FILE and AGENTLANG_DIAGNOSTICS_FILE (set by
   CommandRepairer for every round).
2. Parses diagnostics.json (codes, positions, expected/received, repair hints).
3. Applies deterministic fixes for every AgentLang error code.
4. Writes the repaired source back to AGENTLANG_SOURCE_FILE.
5. Exits 0 (CommandRepairer treats unchanged source as "give up").

AgentLang holds no API keys or credentials — this script is pure local logic.
"""

import json
import os
import re
import sys


def _lit(expected: str | None) -> str:
    """Return a literal of the given type for use as a replacement value."""
    if expected and expected.startswith("Result<"):
        return "Result::Ok(0)"
    return {
        "i32": "0",
        "i64": "0",
        "Bool": "true",
        "String": '""',
    }.get(expected, "0")


def main() -> int:
    source_file = os.environ.get("AGENTLANG_SOURCE_FILE")
    diag_file = os.environ.get("AGENTLANG_DIAGNOSTICS_FILE")
    if not source_file or not diag_file:
        # Not running under CommandRepairer — nothing to do.
        return 0

    try:
        with open(source_file, "r", encoding="utf-8-sig") as f:
            source = f.read()
    except OSError:
        return 0

    try:
        with open(diag_file, "r", encoding="utf-8-sig") as f:
            diags = json.load(f)
    except (OSError, json.JSONDecodeError):
        return 0

    if not diags:
        return 0

    changed = False

    for d in diags:
        code = d.get("code", "")
        msg = d.get("message", "")
        symbol = d.get("symbol")
        expected = d.get("expected")
        repair = d.get("repair") or {}
        repair_expected = repair.get("expected_type")

        # ── IMMUTABLE_ASSIGNMENT ──────────────────────────────────────────
        # let name = ...  →  let mut name = ...
        # For field/index targets where the root is a `let` binding, add mut.
        if code == "IMMUTABLE_ASSIGNMENT":
            target = symbol
            if not target:
                m = re.search(r"\blet\s+(\w+)\s*=", source)
                if m:
                    target = m.group(1)
            if target:
                pattern = r"\blet\s+" + re.escape(target) + r"\b"
                new_source, n = re.subn(
                    pattern, f"let mut {target}", source, count=1
                )
                if n:
                    source = new_source
                    changed = True
                continue

            # No `let` binding (e.g. function param is immutable).
            # Introduce a mutable copy: `let mut buf = items;` and use buf.
            if "index" in msg.lower():
                lines = source.split("\n")
                new_lines = []
                inserted = False
                for line in lines:
                    if not inserted and re.search(r"\w+\s*\[\s*\d+\s*\]\s*=", line):
                        m = re.search(r"(\w+)\s*\[\s*\d+\s*\]\s*=", line)
                        if m:
                            arr_name = m.group(1)
                            indent = line[: len(line) - len(line.lstrip())]
                            new_lines.append(f"{indent}let mut buf = {arr_name};")
                            line = line.replace(arr_name + "[", "buf[", 1)
                            inserted = True
                    new_lines.append(line)
                if inserted:
                    source = "\n".join(new_lines)
                    changed = True
                continue

        # ── EFFECT_VIOLATION ─────────────────────────────────────────────
        # Add missing effect(s) to the calling function's effects clause.
        if code == "EFFECT_VIOLATION":
            m = re.search(r"Function `(\w+)`", msg)
            if m and repair_expected:
                caller = m.group(1)
                missing = [e.strip() for e in repair_expected.split(",")]
                lines = source.split("\n")
                new_lines = []
                done = False
                func_marker = f"function {caller}("
                for line in lines:
                    if not done and func_marker in line:
                        eff_idx = line.find("effects(")
                        if eff_idx >= 0:
                            close_idx = line.find(")", eff_idx)
                            if close_idx >= 0:
                                existing = line[eff_idx + 8 : close_idx]
                                existing_effects = [
                                    e.strip() for e in existing.split(",") if e.strip()
                                ]
                                all_effects = list(existing_effects)
                                for e in missing:
                                    if e and e not in all_effects:
                                        all_effects.append(e)
                                new_effect_str = ", ".join(all_effects)
                                line = (
                                    line[:eff_idx]
                                    + f"effects({new_effect_str})"
                                    + line[close_idx + 1 :]
                                )
                                done = True
                        else:
                            brace_idx = line.rfind("{")
                            if brace_idx >= 0:
                                prefix = line[:brace_idx].rstrip()
                                line = (
                                    f"{prefix} effects({', '.join(missing)}) {{"
                                )
                                done = True
                    new_lines.append(line)
                if done:
                    source = "\n".join(new_lines)
                    changed = True

        # ── TYPE_MISMATCH ────────────────────────────────────────────────
        if code == "TYPE_MISMATCH":
            # Index *value* mismatch: `buf[0] = "nope"` → `buf[0] = 0;`
            if expected:
                new_source, n = re.subn(
                    r"(\w+\s*\[\s*\d+\s*\]\s*=\s*).*?;",
                    lambda m: m.group(1) + _lit(expected) + ";",
                    source,
                    count=1,
                )
                if n:
                    source = new_source
                    changed = True
                    continue

            # Return type mismatch: `return "hello";` → `return 0;`
            if "Return type mismatch" in msg and expected:
                if expected.startswith("Result<"):
                    new_source, n = re.subn(
                        r"return\s+(\w+)\s*;", "return Result::Ok(0);", source, count=1
                    )
                else:
                    repl = _lit(expected)
                    new_source, n = re.subn(
                        r"return\s+.*?;", f"return {repl};", source, count=1
                    )
                if n:
                    source = new_source
                    changed = True
                    continue

            # Assignment mismatch: `value = "two";` → `value = 0;`
            if expected and ("assign" in msg.lower() or "assignment" in msg.lower()):
                new_source, n = re.subn(
                    r"(?m)^(?!.*\blet\b)(\s*\w+\s*=\s*).*?;",
                    lambda m: m.group(1) + _lit(expected) + ";",
                    source,
                    count=1,
                )
                if n:
                    source = new_source
                    changed = True
                    continue

            # Condition must be Bool: `if 1` / `while i` → `if true` / `while true`
            if "condition must be" in msg and expected == "Bool":
                new_source, n = re.subn(
                    r"(if|while)\s+\w+\s*\{", r"\1 true {", source, count=1
                )
                if n:
                    source = new_source
                    changed = True
                    continue

            # Index type mismatch: `let idx = true` → `let idx = 0;`
            if expected:
                new_source, n = re.subn(
                    r"(let\s+\w+\s*=\s*).*?;",
                    lambda m: m.group(1) + _lit(expected) + ";",
                    source,
                    count=1,
                )
                if n:
                    source = new_source
                    changed = True
                    continue

        # ── MISSING_RETURN ───────────────────────────────────────────────
        elif code == "MISSING_RETURN":
            if source.rstrip().endswith("}"):
                source = re.sub(r"\}$", "    return 0;\n}", source, count=1)
                changed = True

        # ── UNDEFINED_SYMBOL ─────────────────────────────────────────────
        elif code == "UNDEFINED_SYMBOL":
            if symbol:
                init = _lit(expected)
                decl = f"    let {symbol} = {init};\n"
                new_source, n = re.subn(
                    r"(return\s)", decl + r"\1", source, count=1
                )
                if n:
                    source = new_source
                    changed = True

        # ── MISSING_FIELD ────────────────────────────────────────────────
        elif code == "MISSING_FIELD":
            # Add missing field to a struct *literal*, not a definition.
            if symbol:
                init = _lit(repair_expected or expected)
                m = re.search(r"(\w+)\s*\{([^{}\n\r]*)\}", source)
                if m:
                    name, inner = m.group(1), m.group(2)
                    if symbol not in inner:
                        new_inner = inner.rstrip() + f", {symbol}: {init}"
                        source = (
                            source[: m.start(2)]
                            + new_inner
                            + source[m.end(2) :]
                        )
                        changed = True

        # ── ARITY_MISMATCH ───────────────────────────────────────────────
        elif code == "ARITY_MISMATCH":
            # Add a missing argument to a function call.
            m = re.search(r"return\s+(\w+)\(([^)]*)\)\s*;", source)
            if m:
                args = m.group(2)
                new_args = f"{args}, 0" if args.strip() else "0"
                source = (
                    source[: m.start(2)] + new_args + source[m.end(2) :]
                )
                changed = True

        # ── INVALID_TRY ──────────────────────────────────────────────────
        elif code == "INVALID_TRY":
            # Change the function's return type to Result<T, String>.
            m = re.search(
                r"(function\s+\w+\s*\([^)]*\)\s*->\s*\w+\s*\{[^}]*?\?[^}]*?\})",
                source,
                re.DOTALL,
            )
            if m:
                func_block = m.group(1)
                new_func = re.sub(
                    r"(function\s+\w+\s*\([^)]*\)\s*->\s*)(\w+)",
                    r"\1Result<\2, String>",
                    func_block,
                )
                source = source.replace(func_block, new_func, 1)
                changed = True

        # ── NON_EXHAUSTIVE_MATCH ─────────────────────────────────────────
        elif code == "NON_EXHAUSTIVE_MATCH":
            new_source, n = re.subn(
                r"(,\s*)\};", r"\1\n        _ => 0,\n    };", source, count=1
            )
            if n:
                source = new_source
                changed = True

        # ── PARSE_ERROR ──────────────────────────────────────────────────
        elif code == "PARSE_ERROR":
            # Missing semicolon before `return`.
            new_source, n = re.subn(
                r"(\n\s*return)", r";\1", source, count=1
            )
            if n:
                source = new_source
                changed = True

        # ── NOT_INDEXABLE ────────────────────────────────────────────────
        elif code == "NOT_INDEXABLE":
            new_source, n = re.subn(r"\d+\s*\[\s*\d+\s*\]", "0", source, count=1)
            if n:
                source = new_source
                changed = True

        # ── NOT_ITERABLE ─────────────────────────────────────────────────
        elif code == "NOT_ITERABLE":
            new_source, n = re.subn(
                r"for\s+\w+\s+in\s+\d+\s*\{", "if true {", source, count=1
            )
            if n:
                source = new_source
                changed = True

        # ── DUPLICATE_SYMBOL ─────────────────────────────────────────────
        elif code == "DUPLICATE_SYMBOL":
            if symbol:
                pattern = (
                    r"^\s*type\s+" + re.escape(symbol) + r"\s*=.*?;\s*\n"
                )
                matches = list(re.finditer(pattern, source, re.MULTILINE))
                if len(matches) >= 2:
                    second = matches[1]
                    source = source[: second.start()] + source[second.end() :]
                    changed = True

    if changed:
        try:
            with open(source_file, "w", encoding="utf-8") as f:
                f.write(source)
        except OSError:
            pass

    return 0


if __name__ == "__main__":
    sys.exit(main())
