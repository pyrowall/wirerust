// Integration tests for wirerust: end-to-end filter parsing, compilation, and execution

use wirerust::*;

fn make_schema() -> FilterSchema {
    FilterSchemaBuilder::new()
        .field("http.method", FieldType::Bytes)
        .field("port", FieldType::Int)
        .field("tags", FieldType::Array(Box::new(FieldType::Bytes)))
        .build()
}

fn make_functions() -> FunctionRegistry {
    let mut functions = FunctionRegistry::new();
    register_builtins(&mut functions);
    functions
}

#[test]
fn test_filter_matches() {
    let schema = make_schema();
    let functions = make_functions();
    let filter_str = r#"http.method == "GET" && port in {80 443} && len(tags) == 2"#;
    let expr = FilterParser::parse(filter_str, &schema).expect("parse");
    let filter = CompiledFilter::new(expr, schema.clone(), functions.clone());

    let mut ctx = FilterContext::new();
    ctx.set("http.method", LiteralValue::Bytes(b"GET".to_vec()), &schema).unwrap();
    ctx.set("port", LiteralValue::Int(80), &schema).unwrap();
    ctx.set(
        "tags",
        LiteralValue::Array(vec![
            LiteralValue::Bytes(b"foo".to_vec()),
            LiteralValue::Bytes(b"bar".to_vec()),
        ]),
        &schema,
    ).unwrap();

    assert!(filter.execute(&ctx));
}

#[test]
fn test_filter_does_not_match() {
    let schema = make_schema();
    let functions = make_functions();
    let filter_str = r#"http.method == "POST" || port == 22"#;
    let expr = FilterParser::parse(filter_str, &schema).expect("parse");
    let filter = CompiledFilter::new(expr, schema.clone(), functions.clone());

    let mut ctx = FilterContext::new();
    ctx.set("http.method", LiteralValue::Bytes(b"GET".to_vec()), &schema).unwrap();
    ctx.set("port", LiteralValue::Int(80), &schema).unwrap();
    assert!(!filter.execute(&ctx));
}

#[test]
fn test_filter_with_function_call() {
    let schema = make_schema();
    let functions = make_functions();
    let filter_str = r#"upper(http.method) == "GET""#;
    let expr = FilterParser::parse(filter_str, &schema).expect("parse");
    let filter = CompiledFilter::new(expr, schema.clone(), functions.clone());

    let mut ctx = FilterContext::new();
    ctx.set("http.method", LiteralValue::Bytes(b"get".to_vec()), &schema).unwrap();
    ctx.set("port", LiteralValue::Int(80), &schema).unwrap();
    ctx.set(
        "tags",
        LiteralValue::Array(vec![
            LiteralValue::Bytes(b"foo".to_vec()),
            LiteralValue::Bytes(b"bar".to_vec()),
        ]),
        &schema,
    ).unwrap();

    assert!(filter.execute(&ctx));
} 