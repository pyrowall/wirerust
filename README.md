# Wirerust

A modular, embeddable filter engine for structured data, inspired by Cloudflare's wirefilter but redesigned for general-purpose use, extensibility, and modern Rust idioms.

## Features

- **Schema Definition**: Define fields and types for your data.
- **Expression Parsing**: Parse filter expressions into an AST.
- **Compilation & Execution**: Compile expressions to efficient closures and execute against runtime data.
- **Extensible Functions**: Register built-in or custom filter functions.
- **Type System**: Supports bytes, int, bool, IP, arrays, and maps.
- **Embeddable**: Designed for easy integration into Rust projects.
- **Serialization**: Schemas, contexts, and expressions are serializable via Serde.

## Usage Example

```rust
use wirerust::{
    FilterSchemaBuilder, FilterContext, FilterExpr, FilterParser, CompiledFilter, FunctionRegistry, types::FieldType, types::LiteralValue
};

// Define schema
let schema = FilterSchemaBuilder::new()
    .field("foo", FieldType::Int)
    .field("bar", FieldType::Bytes)
    .build();

// Parse expression
let expr = FilterParser::parse("foo == 42 && bar == \"baz\"", &schema).unwrap();

// Build context
let mut ctx = FilterContext::new();
ctx.set("foo", LiteralValue::Int(42), &schema).unwrap();
ctx.set("bar", LiteralValue::Bytes(b"baz".to_vec()), &schema).unwrap();

// Register functions
let mut functions = FunctionRegistry::new();
wirerust::register_builtins(&mut functions);

// Compile and execute
let filter = CompiledFilter::new(expr, schema, functions);
assert!(filter.execute(&ctx));
```

## Architecture

- **Schema**: Field/type registry.
- **Types**: Strongly-typed values and type inference.
- **Expr**: AST for filter expressions, parser, and visitor.
- **Compiler**: Compiles AST to closures for fast execution.
- **Context**: Runtime value store, type-checked.
- **Functions**: Built-in and user-defined filter functions.

## Status

- Core modules implemented: schema, types, expr, compiler, context, functions.
- Basic logical, comparison, and function call support.
- Extensible function registry.
- Unit tests for all core modules.
- TODO: richer type inference, dynamic operator registration, FFI/WASM, advanced error handling, property-based/fuzz testing, performance benchmarks.

## Contributing

Contributions are welcome! Please open issues or pull requests for bug reports, feature requests, or improvements.

- Run `cargo test` to ensure all tests pass.
- Follow Rust style and idioms.
- Add tests for new features or bug fixes.
- See [CONTRIBUTING.md](CONTRIBUTING.md) for more details (to be created).

## License

MIT or Apache-2.0 