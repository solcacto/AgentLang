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
