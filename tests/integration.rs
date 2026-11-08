use agentlang::check_source;

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
