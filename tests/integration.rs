use agentlang::bench::{load_tasks, run_suite, NoopRepairer, OracleRepairer};
use agentlang::check_source;
use std::path::PathBuf;

fn benchmarks_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks")
}

#[test]
fn clean_program_has_no_diagnostics() {
    let src = r#"
function main() -> i32 {
    let x = 40;
    let y = 2;
    return x + y;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn type_mismatch_reports_repair() {
    let src = r#"
function main() -> i32 {
    return "hello";
}
"#;
    let diags = check_source(src, "test.ag");
    assert_eq!(diags.len(), 1, "got {:?}", diags);
    assert_eq!(diags[0].code, "TYPE_MISMATCH");
    assert!(diags[0].repairable);
    assert_eq!(diags[0].expected.as_deref(), Some("i32"));
    assert_eq!(diags[0].received.as_deref(), Some("String"));
}

#[test]
fn undefined_symbol_detected() {
    let src = r#"
function main() -> i32 {
    return missing;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "UNDEFINED_SYMBOL"),
        "got {:?}",
        diags
    );
}

#[test]
fn function_call_type_checks() {
    let src = r#"
function add(a: i32, b: i32) -> i32 {
    return a + b;
}

function main() -> i32 {
    return add(19, 23);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn arity_mismatch_detected() {
    let src = r#"
function add(a: i32, b: i32) -> i32 {
    return a + b;
}

function main() -> i32 {
    return add(1);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "ARITY_MISMATCH"),
        "got {:?}",
        diags
    );
}

#[test]
fn parse_error_is_structured() {
    // Missing semicolon after the returned expression.
    let src = "function main() -> i32 { return 1 }";
    let diags = check_source(src, "test.ag");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "PARSE_ERROR");
    assert_eq!(diags[0].file, "test.ag");
}

#[test]
fn multiple_parse_errors_in_different_functions() {
    let src = r#"
function a() -> i32 {
    return 1
}

function b() -> i32 {
    return 2
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.len() >= 2,
        "expected multiple parse errors, got {:?}",
        diags
    );
    assert!(diags.iter().all(|d| d.code == "PARSE_ERROR"));
}

#[test]
fn multiple_parse_errors_in_one_function() {
    let src = r#"
function main() -> i32 {
    let x = ;
    let y = ;
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.len() >= 2,
        "expected multiple parse errors, got {:?}",
        diags
    );
    assert!(diags.iter().all(|d| d.code == "PARSE_ERROR"));
}

#[test]
fn error_in_struct_does_not_abort_later_functions() {
    let src = r#"
struct Bad {
    x:
}

function main() -> i32 {
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "PARSE_ERROR"),
        "got {:?}",
        diags
    );
    // A well-formed later function should not itself produce errors.
    assert!(
        !diags.iter().any(|d| d.symbol.as_deref() == Some("main")),
        "unexpected error attributed to main: {:?}",
        diags
    );
}

#[test]
fn struct_literal_and_field_access() {
    let src = r#"
struct Point { x: i32, y: i32 }

function origin() -> Point {
    return Point { x: 0, y: 0 };
}

function main() -> i32 {
    let p = origin();
    return p.x + p.y;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn missing_struct_field_detected() {
    let src = r#"
struct Point { x: i32, y: i32 }

function main() -> i32 {
    let p = Point { x: 0 };
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "MISSING_FIELD"),
        "got {:?}",
        diags
    );
}

#[test]
fn unknown_struct_field_detected() {
    let src = r#"
struct Point { x: i32, y: i32 }

function main() -> i32 {
    let p = Point { x: 0, y: 0, z: 0 };
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "UNKNOWN_FIELD"),
        "got {:?}",
        diags
    );
}

#[test]
fn struct_field_type_mismatch_detected() {
    let src = r#"
struct Point { x: i32, y: i32 }

function main() -> i32 {
    let p = Point { x: "zero", y: 0 };
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "TYPE_MISMATCH"),
        "got {:?}",
        diags
    );
}

#[test]
fn enum_declaration_accepted() {
    let src = r#"
enum Color { Red, Green, Blue }

function main() -> i32 {
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn diagnostics_serialize_to_json() {
    let src = r#"
function main() -> i32 {
    return "hello";
}
"#;
    let diags = check_source(src, "test.ag");
    let json = serde_json::to_value(&diags).expect("serialize");
    let first = &json[0];
    assert_eq!(first["code"], "TYPE_MISMATCH");
    assert_eq!(first["expected"], "i32");
    assert_eq!(first["received"], "String");
    assert_eq!(first["repair"]["type"], "replace_expression");
}

#[test]
fn option_construction_and_try() {
    let src = r#"
struct User { name: String }

function find_user() -> Option<User> {
    return Option::None;
}

function get_name() -> Option<String> {
    let user = find_user()?;
    return Option::Some(user.name);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn result_construction_and_try() {
    let src = r#"
function parse_value(input: i32) -> Result<i32, String> {
    return Result::Ok(input);
}

function compute() -> Result<i32, String> {
    let value = parse_value(10)?;
    return Result::Ok(value + 1);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn map_over_option() {
    let src = r#"
struct User { name: String }

function maybe_name(user: Option<User>) -> Option<String> {
    return user.map(|u| u.name);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn bare_constructors_work() {
    let src = r#"
function make() -> Option<i32> {
    return Some(5);
}

function nothing() -> Option<i32> {
    return None;
}

function ok() -> Result<i32, String> {
    return Ok(1);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn invalid_try_detected() {
    let src = r#"
function parse_value(input: i32) -> Result<i32, String> {
    return Result::Ok(input);
}

function compute() -> i32 {
    let value = parse_value(10)?;
    return value;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "INVALID_TRY"),
        "got {:?}",
        diags
    );
}

#[test]
fn try_error_mismatch_detected() {
    let src = r#"
function parse_value(input: i32) -> Result<i32, String> {
    return Result::Ok(input);
}

function compute() -> Result<i32, Bool> {
    let value = parse_value(10)?;
    return Result::Ok(value);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "TRY_TYPE_MISMATCH"),
        "got {:?}",
        diags
    );
}

#[test]
fn unknown_method_detected() {
    let src = r#"
function main() -> i32 {
    let x = 1;
    return x.frobnicate();
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "UNKNOWN_METHOD"),
        "got {:?}",
        diags
    );
}

#[test]
fn user_enum_variant_construction() {
    let src = r#"
enum Color { Red, Green, Blue }

function red() -> Color {
    return Color::Red;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn enum_variant_with_payload() {
    let src = r#"
enum Shape { Circle(i32), Square(i32) }

function unit_circle() -> Shape {
    return Shape::Circle(1);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn unknown_enum_variant_detected() {
    let src = r#"
enum Color { Red, Green, Blue }

function bad() -> Color {
    return Color::Purple;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "UNKNOWN_VARIANT"),
        "got {:?}",
        diags
    );
}

#[test]
fn result_unwrap_or_type_checks() {
    let src = r#"
function parse_value(input: i32) -> Result<i32, String> {
    return Result::Ok(input);
}

function main() -> i32 {
    let r = parse_value(1);
    return r.unwrap_or(0);
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn benchmark_suite_resolves_with_oracle() {
    let tasks = load_tasks(&benchmarks_dir()).expect("load benchmarks");
    assert!(!tasks.is_empty(), "expected at least one benchmark task");

    let mut repairer = OracleRepairer;
    let report = run_suite(&tasks, &mut repairer, 5);

    assert_eq!(report.total, tasks.len());
    assert_eq!(
        report.unresolved, 0,
        "oracle repairer should resolve every task: {:?}",
        report.tasks
    );
    assert!(
        report.first_pass_ok >= 1,
        "expected the baseline task to already be clean"
    );
    assert_eq!(report.fixed, report.total);
}

#[test]
fn noop_repairer_leaves_broken_tasks_unresolved() {
    let tasks = load_tasks(&benchmarks_dir()).expect("load benchmarks");

    let mut repairer = NoopRepairer;
    let report = run_suite(&tasks, &mut repairer, 5);

    // Every task except the already-clean baseline should remain broken.
    assert_eq!(report.first_pass_ok, 1);
    assert_eq!(report.unresolved, report.total - report.first_pass_ok);
    assert_eq!(report.avg_iterations, 0.0);
}

#[test]
fn match_on_user_enum() {
    let src = r#"
enum Shape {
    Circle(i32),
    Square(i32),
}

function area(shape: Shape) -> i32 {
    return match shape {
        Shape::Circle(radius) => radius * radius * 3,
        Shape::Square(side) => side * side,
    };
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn match_on_option_and_result() {
    let src = r#"
function unwrap_or_zero(value: Option<i32>) -> i32 {
    return match value {
        Option::Some(v) => v,
        Option::None => 0,
    };
}

function result_or_zero(value: Result<i32, String>) -> i32 {
    return match value {
        Ok(v) => v,
        _ => 0,
    };
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn wildcard_makes_match_exhaustive() {
    let src = r#"
enum Color { Red, Green, Blue }

function label(c: Color) -> i32 {
    return match c {
        Color::Red => 1,
        _ => 0,
    };
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn non_exhaustive_match_detected() {
    let src = r#"
enum Color { Red, Green, Blue }

function label(c: Color) -> i32 {
    return match c {
        Color::Red => 1,
    };
}
"#;
    let diags = check_source(src, "test.ag");
    let missing: Vec<&str> = diags
        .iter()
        .filter(|d| d.code == "NON_EXHAUSTIVE_MATCH")
        .filter_map(|d| d.symbol.as_deref())
        .collect();
    assert!(missing.contains(&"Green"), "got {:?}", diags);
    assert!(missing.contains(&"Blue"), "got {:?}", diags);
}

#[test]
fn duplicate_match_arm_detected() {
    let src = r#"
enum Color { Red, Green }

function label(c: Color) -> i32 {
    return match c {
        Color::Red => 1,
        Color::Red => 2,
        Color::Green => 3,
    };
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "DUPLICATE_MATCH_ARM"),
        "got {:?}",
        diags
    );
}

#[test]
fn match_arm_type_mismatch_detected() {
    let src = r#"
enum Color { Red, Green }

function label(c: Color) -> i32 {
    return match c {
        Color::Red => 1,
        Color::Green => "green",
    };
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "TYPE_MISMATCH" && d.line == 7),
        "got {:?}",
        diags
    );
}

#[test]
fn unknown_variant_in_pattern_detected() {
    let src = r#"
enum Color { Red, Green }

function label(c: Color) -> i32 {
    return match c {
        Color::Purple => 1,
        _ => 0,
    };
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "UNKNOWN_VARIANT"),
        "got {:?}",
        diags
    );
}

#[test]
fn match_payload_binding_has_variant_type() {
    let src = r#"
enum Shape { Circle(String) }

function area(shape: Shape) -> i32 {
    return match shape {
        Shape::Circle(radius) => radius,
    };
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "TYPE_MISMATCH"),
        "expected a type mismatch using a String payload as i32, got {:?}",
        diags
    );
}

#[test]
fn match_variant_pattern_requires_payload_binding() {
    let src = r#"
enum Shape { Circle(i32) }

function area(shape: Shape) -> i32 {
    return match shape {
        Shape::Circle => 1,
    };
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "ARITY_MISMATCH"),
        "got {:?}",
        diags
    );
}

#[test]
fn bench_report_includes_metadata() {
    let tasks = load_tasks(&benchmarks_dir()).expect("load benchmarks");
    let mut repairer = OracleRepairer;
    let report = run_suite(&tasks, &mut repairer, 5);

    assert_eq!(report.metadata.tool, "agentlang");
    assert_eq!(report.metadata.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(report.metadata.repairer, "oracle");
    assert_eq!(report.metadata.max_iters, 5);
}

#[test]
fn command_repairer_edits_source_in_place() {
    use agentlang::bench::{CommandRepairer, Repairer, Task};

    let dir = std::env::temp_dir().join(format!("agentlang-cmd-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create test dir");
    let script = if cfg!(windows) {
        dir.join("repair.cmd")
    } else {
        dir.join("repair.sh")
    };
    if cfg!(windows) {
        std::fs::write(
            &script,
            "@echo off\r\necho.>> \"%AGENTLANG_SOURCE_FILE%\"\r\n",
        )
        .expect("write script");
    } else {
        std::fs::write(&script, "#!/bin/sh\necho >> \"$AGENTLANG_SOURCE_FILE\"\n")
            .expect("write script");
    }
    let command = if cfg!(windows) {
        script.display().to_string()
    } else {
        format!("sh \"{}\"", script.display())
    };

    let broken = "function main() -> i32 {\n    return \"bad\";\n}\n";
    let task = Task {
        name: "cmd_test".to_string(),
        broken: broken.to_string(),
        fixed: String::new(),
    };
    let diags = check_source(broken, "test.ag");
    let mut repairer = CommandRepairer::new(command);
    let repaired = repairer
        .repair(&task, broken, &diags)
        .expect("command should modify the source");

    assert!(
        repaired.starts_with(broken),
        "expected the original source plus an appended line, got {:?}",
        repaired
    );
    assert!(repaired.len() > broken.len());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn command_repairer_returns_none_when_source_is_unchanged() {
    use agentlang::bench::{CommandRepairer, Repairer, Task};

    let command = if cfg!(windows) {
        "exit /b 0".to_string()
    } else {
        "true".to_string()
    };
    let source = "function main() -> i32 {\n    return 0;\n}\n";
    let task = Task {
        name: "noop".to_string(),
        broken: source.to_string(),
        fixed: String::new(),
    };
    let diags = check_source(source, "test.ag");
    let mut repairer = CommandRepairer::new(command);

    assert!(repairer.repair(&task, source, &diags).is_none());
}

#[test]
fn cli_check_format_json_always_emits_json() {
    use std::process::Command;

    let exe = env!("CARGO_BIN_EXE_agent");
    let dir = std::env::temp_dir().join(format!("agentlang-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create test dir");

    let bad = dir.join("bad.ag");
    std::fs::write(&bad, "function main() -> i32 {\n    return \"nope\";\n}\n")
        .expect("write bad source");
    let output = Command::new(exe)
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg(&bad)
        .output()
        .expect("run agent check");
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert!(json
        .as_array()
        .expect("array")
        .iter()
        .any(|diag| diag["code"] == "TYPE_MISMATCH"));

    let good = dir.join("good.ag");
    std::fs::write(&good, "function main() -> i32 {\n    return 0;\n}\n")
        .expect("write good source");
    let output = Command::new(exe)
        .arg("check")
        .arg("--json")
        .arg(&good)
        .output()
        .expect("run agent check");
    assert!(output.status.success());
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(json, serde_json::json!([]));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn if_else_and_else_if_type_check() {
    let src = r#"
function classify(value: i32) -> i32 {
    if value > 0 {
        return 1;
    } else if value < 0 {
        return -1;
    } else {
        return 0;
    }
}

function abs(value: i32) -> i32 {
    if value < 0 {
        return -value;
    }
    return value;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn if_condition_must_be_bool() {
    let src = r#"
function main() -> i32 {
    if 1 {
        return 1;
    }
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "TYPE_MISMATCH" && d.expected.as_deref() == Some("Bool")),
        "got {:?}",
        diags
    );
}

#[test]
fn if_without_else_does_not_guarantee_return() {
    let src = r#"
function check(value: i32) -> i32 {
    if value > 0 {
        return 1;
    }
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "MISSING_RETURN"),
        "got {:?}",
        diags
    );
}

#[test]
fn if_with_both_branches_returning_satisfies_return_check() {
    let src = r#"
function check(value: i32) -> i32 {
    if value > 0 {
        return 1;
    } else {
        return 0;
    }
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        !diags.iter().any(|d| d.code == "MISSING_RETURN"),
        "got {:?}",
        diags
    );
}

#[test]
fn branch_scopes_do_not_leak() {
    let src = r#"
function main(flag: Bool) -> i32 {
    if flag {
        let hidden = 1;
    }
    return hidden;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "UNDEFINED_SYMBOL" && d.symbol.as_deref() == Some("hidden")),
        "got {:?}",
        diags
    );
}

#[test]
fn nested_if_and_early_return_type_check() {
    let src = r#"
function pick(a: i32, b: i32) -> i32 {
    if a > 0 {
        if b > 0 {
            return 1;
        } else {
            return 2;
        }
    }
    return 3;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn return_type_is_checked_inside_branches() {
    let src = r#"
function bad(value: i32) -> i32 {
    if value > 0 {
        return "positive";
    } else {
        return 0;
    }
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "TYPE_MISMATCH"),
        "got {:?}",
        diags
    );
}

#[test]
fn mutable_assignment_and_while_type_check() {
    let src = r#"
function sum_to(limit: i32) -> i32 {
    let mut total = 0;
    let mut i = 0;
    while i < limit {
        total = total + i;
        i = i + 1;
    }
    return total;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn assignment_to_immutable_binding_is_rejected() {
    let src = r#"
function main() -> i32 {
    let value = 1;
    value = 2;
    return value;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "IMMUTABLE_ASSIGNMENT" && d.symbol.as_deref() == Some("value")),
        "got {:?}",
        diags
    );
}

#[test]
fn assignment_type_mismatch_is_rejected() {
    let src = r#"
function main() -> i32 {
    let mut value = 1;
    value = "two";
    return value;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "TYPE_MISMATCH"),
        "got {:?}",
        diags
    );
}

#[test]
fn assignment_to_undefined_symbol_is_rejected() {
    let src = r#"
function main() -> i32 {
    missing = 1;
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "UNDEFINED_SYMBOL" && d.symbol.as_deref() == Some("missing")),
        "got {:?}",
        diags
    );
}

#[test]
fn while_condition_must_be_bool() {
    let src = r#"
function main() -> i32 {
    let mut i = 0;
    while i {
        i = i + 1;
    }
    return i;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "TYPE_MISMATCH" && d.expected.as_deref() == Some("Bool")),
        "got {:?}",
        diags
    );
}

#[test]
fn while_body_scope_does_not_leak() {
    let src = r#"
function main(flag: Bool) -> i32 {
    while flag {
        let inside = 1;
    }
    return inside;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "UNDEFINED_SYMBOL" && d.symbol.as_deref() == Some("inside")),
        "got {:?}",
        diags
    );
}

#[test]
fn mutable_binding_can_be_updated_from_nested_blocks() {
    let src = r#"
function main() -> i32 {
    let mut total = 0;
    if true {
        total = 1;
    }
    while total < 3 {
        total = total + 1;
    }
    return total;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn while_does_not_guarantee_return() {
    let src = r#"
function spin() -> i32 {
    while true {
        return 1;
    }
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "MISSING_RETURN"),
        "got {:?}",
        diags
    );
}

#[test]
fn for_over_array_binds_element_type() {
    let src = r#"
function sum(items: Array<i32>) -> i32 {
    let mut total = 0;
    for x in items {
        total = total + x;
    }
    return total;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn for_over_map_binds_key_type() {
    let src = r#"
function count_keys(scores: Map<String, i32>) -> i32 {
    let mut n = 0;
    for name in scores {
        n = n + 1;
    }
    return n;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn for_over_non_iterable_reports_not_iterable() {
    let src = r#"
function main() -> i32 {
    for x in 42 {
        return 1;
    }
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "NOT_ITERABLE"),
        "got {:?}",
        diags
    );
}

#[test]
fn index_array_requires_i32() {
    let src = r#"
function main(items: Array<i32>) -> i32 {
    let idx = true;
    return items[idx];
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "TYPE_MISMATCH" && d.expected.as_deref() == Some("i32")),
        "got {:?}",
        diags
    );
}

#[test]
fn index_map_requires_key_type() {
    let src = r#"
function main(scores: Map<String, i32>) -> i32 {
    return scores[0];
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "TYPE_MISMATCH" && d.expected.as_deref() == Some("String")),
        "got {:?}",
        diags
    );
}

#[test]
fn index_non_collection_reports_not_indexable() {
    let src = r#"
function main() -> i32 {
    return 42[0];
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "NOT_INDEXABLE"),
        "got {:?}",
        diags
    );
}

#[test]
fn index_expression_returns_element_type() {
    let src = r#"
function first(items: Array<i32>) -> i32 {
    return items[0];
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn for_loop_variable_is_immutable() {
    let src = r#"
function main(items: Array<i32>) -> i32 {
    for x in items {
        x = 1;
    }
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "IMMUTABLE_ASSIGNMENT" && d.symbol.as_deref() == Some("x")),
        "got {:?}",
        diags
    );
}

#[test]
fn for_does_not_guarantee_return() {
    let src = r#"
function spin(items: Array<i32>) -> i32 {
    for x in items {
        return 1;
    }
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "MISSING_RETURN"),
        "got {:?}",
        diags
    );
}

#[test]
fn for_condition_scope_does_not_leak() {
    let src = r#"
function main(items: Array<i32>) -> i32 {
    for x in items {
        let inner = 1;
    }
    return inner;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "UNDEFINED_SYMBOL" && d.symbol.as_deref() == Some("inner")),
        "got {:?}",
        diags
    );
}

#[test]
fn field_mutation_on_mutable_struct() {
    let src = r#"
struct Point {
    x: i32,
    y: i32,
}
function main() -> i32 {
    let mut p = Point { x: 1, y: 2 };
    p.x = 10;
    return p.x;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn field_mutation_on_immutable_binding_rejected() {
    let src = r#"
struct Point {
    x: i32,
    y: i32,
}
function main() -> i32 {
    let p = Point { x: 1, y: 2 };
    p.x = 10;
    return p.x;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "IMMUTABLE_ASSIGNMENT"),
        "got {:?}",
        diags
    );
}

#[test]
fn index_mutation_on_mutable_array() {
    let src = r#"
function set_first(items: Array<i32>, v: i32) -> i32 {
    let mut buf = items;
    buf[0] = v;
    return buf[0];
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn index_mutation_on_immutable_binding_rejected() {
    let src = r#"
function main(items: Array<i32>) -> i32 {
    items[0] = 1;
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "IMMUTABLE_ASSIGNMENT"),
        "got {:?}",
        diags
    );
}

#[test]
fn index_mutation_requires_i32_index() {
    let src = r#"
function main(items: Array<i32>) -> i32 {
    let mut buf = items;
    let idx = true;
    buf[idx] = 1;
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "TYPE_MISMATCH" && d.expected.as_deref() == Some("i32")),
        "got {:?}",
        diags
    );
}

#[test]
fn index_mutation_value_type_mismatch() {
    let src = r#"
function main(items: Array<i32>) -> i32 {
    let mut buf = items;
    buf[0] = "nope";
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "TYPE_MISMATCH"),
        "got {:?}",
        diags
    );
}

#[test]
fn index_mutation_into_non_collection_rejected() {
    let src = r#"
function main() -> i32 {
    let mut n = 42;
    n[0] = 1;
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "NOT_INDEXABLE"),
        "got {:?}",
        diags
    );
}

#[test]
fn assignment_to_non_lvalue_rejected() {
    let src = r#"
function main() -> i32 {
    42 = 1;
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(!diags.is_empty(), "expected a diagnostic, got {:?}", diags);
}

#[test]
fn type_alias_resolves_to_underlying_type() {
    let src = r#"
type UserId = i32;

function main() -> UserId {
    return 42;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn type_alias_mismatch_rejected() {
    let src = r#"
type UserId = i32;

function main() -> UserId {
    return "not a number";
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "TYPE_MISMATCH"),
        "got {:?}",
        diags
    );
}

#[test]
fn type_alias_in_generic() {
    let src = r#"
type Id = i32;

function sum(ids: Array<Id>) -> Id {
    let mut total = 0;
    for x in ids {
        total = total + x;
    }
    return total;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn type_alias_forward_reference() {
    let src = r#"
type B = A;
type A = i32;

function main() -> B {
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn duplicate_type_alias_rejected() {
    let src = r#"
type Id = i32;
type Id = i32;

function main() -> i32 {
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "DUPLICATE_SYMBOL"),
        "got {:?}",
        diags
    );
}

#[test]
fn type_alias_conflicts_with_struct_rejected() {
    let src = r#"
struct Point { x: i32, y: i32, }
type Point = i32;

function main() -> i32 {
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "DUPLICATE_SYMBOL"),
        "got {:?}",
        diags
    );
}

#[test]
fn type_alias_conflicts_with_function_rejected() {
    let src = r#"
function foo() -> i32 { return 0; }
type foo = i32;

function main() -> i32 {
    return 0;
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| d.code == "DUPLICATE_SYMBOL"),
        "got {:?}",
        diags
    );
}

#[test]
fn effect_enforcement_clean() {
    let src = r#"
function read_db() -> i32 effects(db.read) {
    return 42;
}

function safe_read() -> i32 effects(db.read) {
    return read_db();
}

function main() -> i32 effects(db.read) {
    return safe_read();
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn effect_violation_reported() {
    let src = r#"
function read_db() -> i32 effects(db.read) {
    return 42;
}

function unsafe_read() -> i32 {
    return read_db();
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags
            .iter()
            .any(|d| d.code == "EFFECT_VIOLATION" && d.symbol.as_deref() == Some("read_db")),
        "got {:?}",
        diags
    );
}

#[test]
fn effect_subset_allowed() {
    let src = r#"
function read_db() -> i32 effects(db.read) {
    return 42;
}

function caller() -> i32 effects(db.read, db.write) {
    return read_db();
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
}

#[test]
fn effect_repair_has_add_effect() {
    let src = r#"
function read_db() -> i32 effects(db.read) {
    return 42;
}

function bad() -> i32 {
    return read_db();
}
"#;
    let diags = check_source(src, "test.ag");
    assert!(
        diags.iter().any(|d| {
            d.code == "EFFECT_VIOLATION"
                && d.repair
                    .as_ref()
                    .map(|r| r.kind == "add_effect")
                    .unwrap_or(false)
        }),
        "got {:?}",
        diags
    );
}
