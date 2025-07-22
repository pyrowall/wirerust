use wirerust::*;

fn main() -> Result<(), WirerustError> {
    // 1. Define schema
    let schema = FilterSchemaBuilder::new()
        .field("http.method", FieldType::Bytes)
        .field("port", FieldType::Int)
        .field("tags", FieldType::Array(Box::new(FieldType::Bytes)))
        .build();

    // 2. Register built-in functions
    let mut functions = FunctionRegistry::new();
    register_builtins(&mut functions);

    // 3. Parse filter expression
    let filter_str = r#"http.method == "GET" && port in {80 443} && len(tags) == 2"#;
    let expr = FilterParser::parse(filter_str, &schema)?;
    println!("Parsed AST: {:#?}", expr);

    // 4. Compile filter
    let filter = CompiledFilter::new(expr, schema.clone(), functions);

    // 5. Create context and set values
    let mut ctx = FilterContext::new();
    ctx.set("http.method", LiteralValue::Bytes(b"GET".to_vec()), &schema)?;
    ctx.set("port", LiteralValue::Int(80), &schema)?;
    ctx.set(
        "tags",
        LiteralValue::Array(vec![LiteralValue::Bytes(b"foo".to_vec()), LiteralValue::Bytes(b"bar".to_vec())]),
        &schema,
    )?;

    // 6. Execute filter
    let result = filter.execute(&ctx);
    match result {
        Ok(val) => println!("Filter matches: {}", val),
        Err(e) => println!("Filter error: {}", e),
    }
    Ok(())
} 