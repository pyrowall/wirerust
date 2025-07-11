# Wirerust

Wirerust is a modular, embeddable filter engine for structured data, inspired by Cloudflare's wirefilter but designed for general use, extensibility, and modern Rust idioms.

## Goals
- Clean, idiomatic Rust API
- Extensible function and type system
- Pluggable execution backends
- Optional FFI and WASM bindings
- No Cloudflare or Wireshark-specific dependencies

## Architecture Overview
- **Schema**: Define available fields and types
- **Expression Parsing**: Parse filter strings into an AST
- **Compilation**: Compile AST into an intermediate representation (IR)
- **Execution Context**: Provide runtime values for fields
- **Evaluation**: Execute compiled filters against context

## Migration Checklist from wirefilter

### 1. Core Types & Modules
- [x] Scaffold main modules: schema, expr, compiler, filter, context, types, functions
- [x] Implement schema definition and builder
- [x] Implement AST and parser for filter expressions
- [x] Implement compiler trait and default backend
- [x] Implement execution context for runtime values
- [x] Implement filter execution logic
- [x] Implement extensible function/type registries
- [x] Support function calls in expressions
- [x] Support list/set literals in expressions

### 2. API Modernization
- [x] Rename types and modules for clarity (see design doc)
- [x] Use builder and strategy patterns where appropriate
- [x] Add comprehensive Rustdoc documentation

### 3. Extensibility & Modularity
- [x] Make function/type registries pluggable
- [x] Abstract execution backend (trait for IR execution)
- [x] Add feature flags for regex, simd, ffi, wasm, serde

### 4. Bindings & Examples
- [ ] Add FFI bindings (optional)
- [ ] Add WASM bindings (optional)
- [x] Add examples and integration tests

### 5. Next Steps
- [x] Improve error handling and diagnostics
- [x] Add serde derives for serialization support
- [x] Add more built-in functions
- [x] Add more comprehensive tests and usage examples

---

## Basic Usage Example

```rust
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
    let filter_str = r#"http.method == \"GET\" && port in {80 443} && len(tags) == 2"#;
    let expr = FilterParser::parse(filter_str, &schema)?;
    println!("Parsed AST: {:#?}", expr);

    // 4. Compile filter
    let filter = CompiledFilter::new(&expr, schema.clone(), &functions);

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
    println!("Filter matches: {}", result);
    Ok(())
}
```

---

The crate is ready for integration and further extension. See the migration checklist for remaining optional tasks. 