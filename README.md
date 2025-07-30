# Wirerust

[![Crates.io](https://img.shields.io/crates/v/wirerust)](https://crates.io/crates/wirerust)
[![Documentation](https://docs.rs/wirerust/badge.svg)](https://docs.rs/wirerust)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.88.0+-blue.svg)](https://www.rust-lang.org)

A high-performance, modular filter engine for structured data written in Rust. Wirerust provides a clean, extensible implementation for parsing, compiling, and executing filter expressions against runtime data with strong type safety and excellent performance.

## Features

✨ **Core Capabilities**
- **Schema-driven filtering**: Define field types and constraints for your data
- **Expression parsing**: Parse human-readable filter expressions into an AST
- **Compilation**: Compile expressions to efficient closures for fast execution
- **Type safety**: Strong type checking with comprehensive error handling
- **Extensible functions**: Register built-in or custom filter functions
- **Serialization**: Full Serde support for schemas, contexts, and expressions

🔧 **Supported Types**
- **Primitives**: `bool`, `int`, `bytes`, `string`
- **Network**: `ip` (IPv4/IPv6 addresses)
- **Collections**: `array`, `map` with type inference
- **Special**: `unknown` for dynamic typing

⚡ **Performance**
- Zero-copy parsing where possible
- Compiled expressions for optimal runtime performance
- Efficient memory usage with Arc-based sharing
- Property-based testing ensures correctness

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
wirerust = "0.1.0"
```

Basic usage:

```rust
use wirerust::{
    WirerustEngineBuilder, FilterContext, LiteralValue, FieldType
};

// Create an engine with schema
let engine = WirerustEngineBuilder::new()
    .field("status", FieldType::Int)
    .field("method", FieldType::Bytes)
    .field("path", FieldType::Bytes)
    .build();

// Parse and compile a filter
let filter = engine.parse_and_compile("status == 200 && method == \"GET\"").unwrap();

// Create context with data
let mut ctx = FilterContext::new();
ctx.set("status", LiteralValue::Int(200), engine.schema()).unwrap();
ctx.set("method", LiteralValue::Bytes(b"GET".to_vec()), engine.schema()).unwrap();

// Execute the filter
assert!(filter.execute(&ctx).unwrap());
```

## Advanced Usage

### Custom Functions

```rust
use wirerust::{FilterFunction, LiteralValue, WirerustEngineBuilder, FieldType};

struct CustomFilter;

impl FilterFunction for CustomFilter {
    fn call(&self, args: &[LiteralValue]) -> Option<LiteralValue> {
        // Your custom logic here
        Some(LiteralValue::Bool(true))
    }
}

let engine = WirerustEngineBuilder::new()
    .field("data", FieldType::Bytes)
    .register_function("custom_filter", CustomFilter)
    .build();
```

### Complex Expressions

```rust
// IP address filtering
let filter = engine.parse_and_compile(
    "ip in [\"192.168.1.1\", \"10.0.0.1\"] && status >= 200 && status < 300"
).unwrap();

// String operations with functions
let filter = engine.parse_and_compile(
    "starts_with(path, \"/api/\") && len(path) > 10"
).unwrap();

// Logical combinations
let filter = engine.parse_and_compile(
    "(method == \"POST\" || method == \"PUT\") && status != 404"
).unwrap();
```

## Built-in Functions

| Function | Description | Example |
|----------|-------------|---------|
| `len()` | Get length of string/array | `len(name) > 5` |
| `starts_with()` | Check string prefix | `starts_with(path, "/api/")` |
| `ends_with()` | Check string suffix | `ends_with(filename, ".json")` |
| `sum()` | Sum array of numbers | `sum(scores) > 100` |
| `upper()` | Convert to uppercase | `upper(method) == "GET"` |

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Filter String  │───▶│ AST (FilterExpr)│───▶│  CompiledFilter │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │                      │
                                ▼                      ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │   Type Checker  │    │    Execution    │
                       └─────────────────┘    └─────────────────┘
                                │                      │
                                ▼                      ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │     Schema      │    │     Context     │
                       └─────────────────┘    └─────────────────┘
```

### Core Components

- **Schema**: Field definitions and type constraints
- **Parser**: Converts filter strings to AST
- **Compiler**: Optimizes AST to executable closures
- **Context**: Runtime value storage with type checking
- **Functions**: Extensible function registry
- **Types**: Strong type system with inference

## Performance

Wirerust is designed for high-performance scenarios:

- **Zero-copy parsing** where possible
- **Compiled expressions** avoid repeated parsing
- **Efficient memory usage** with smart pointer sharing
- **Type-safe execution** prevents runtime errors
- **Property-based testing** ensures correctness

Benchmarks show excellent performance for typical filter operations.

## Development

### Prerequisites

- Rust 1.88.0 or later
- Git

### Setup

```bash
git clone https://github.com/pyrowall/wirerust.git
cd wirerust
cargo test
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with all features
cargo test --all-features

# Run benchmarks
cargo bench

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

### Version Management

```bash
# Bump patch version (0.1.0 -> 0.1.1)
./scripts/bump-version.sh patch

# Bump minor version (0.1.0 -> 0.2.0)
./scripts/bump-version.sh minor

# Bump major version (0.1.0 -> 1.0.0)
./scripts/bump-version.sh major
```

## Roadmap

### Current Status ✅
- Core filter engine with schema support
- Expression parsing and compilation
- Type-safe execution context
- Built-in function library
- Comprehensive test suite
- Property-based testing
- Serialization support

### Planned Features 🚧
- **Enhanced type inference** for arrays and maps
- **Dynamic operator registration** for extensibility
- **FFI/WASM bindings** for cross-language use
- **Advanced error handling** with detailed diagnostics
- **Performance benchmarks** and optimization
- **Query optimization** for complex expressions
- **Caching layer** for compiled filters

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Guidelines

1. **Tests**: Ensure all tests pass with `cargo test`
2. **Style**: Follow Rust idioms and run `cargo clippy`
3. **Documentation**: Add docs for new features
4. **Commits**: Use conventional commit messages

### Getting Started

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Run the full test suite
6. Submit a pull request

## License

This project is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- Inspired by Cloudflare's [wirefilter](https://github.com/cloudflare/wirefilter)
- Built with modern Rust idioms and best practices
- Community-driven development and feedback

---

**Wirerust** - Fast, safe, and extensible filtering for Rust applications.