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
