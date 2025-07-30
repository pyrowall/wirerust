//! Wirerust: A modular, embeddable filter engine for structured data.
//!
//! This crate is a clean, extensible reimplementation of the core ideas from Cloudflare's wirefilter.
//! It provides a way to define filter schemas, parse filter expressions, compile them, and execute them against runtime data.
//!
//! # Planned Architecture
//! - Schema definition (fields/types)
//! - Expression parsing (AST)
//! - Compilation to IR (closures or pluggable backends)
//! - Execution context (runtime values)
//! - Extensible function/type registry
//! - Optional FFI/WASM bindings

use std::sync::Arc;
use thiserror::Error;

mod compiler;
mod context;
mod expr;
mod filter;
mod functions;
mod ir;
mod schema;
mod types;

pub use compiler::*;
pub use context::*;
pub use expr::*;
pub use filter::*;
pub use functions::*;
pub use schema::*;
pub use types::*;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WirerustError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Type error: {0}")]
    TypeError(String),
    #[error("Function error: {0}")]
    FunctionError(String),
    #[error("Field not found: {0}")]
    FieldNotFound(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Error: {0}")]
    Other(String),
}

/// The main engine for parsing, compiling, and executing filters.
pub struct WirerustEngine {
    schema: Arc<FilterSchema>,
    functions: Arc<FunctionRegistry>,
}

impl WirerustEngine {
    /// Create a new engine with the given schema and built-in functions.
    pub fn new(schema: FilterSchema) -> Self {
        let mut functions = FunctionRegistry::new();
        register_builtins(&mut functions);
        Self {
            schema: Arc::new(schema),
            functions: Arc::new(functions),
        }
    }
    /// Create a new engine with the given schema and custom function registry.
    pub fn with_functions(schema: FilterSchema, functions: FunctionRegistry) -> Self {
        Self {
            schema: Arc::new(schema),
            functions: Arc::new(functions),
        }
    }
    /// Get a reference to the filter schema.
    pub fn schema(&self) -> &FilterSchema {
        &self.schema
    }
    /// Get a reference to the function registry.
    pub fn functions(&self) -> &FunctionRegistry {
        &self.functions
    }
    /// Parse a filter expression string into an AST.
    pub fn parse_filter(&self, expr: &str) -> Result<FilterExpr, WirerustError> {
        FilterParser::parse(expr, &self.schema)
    }
    /// Compile a parsed filter expression into an executable filter.
    pub fn compile_filter(&self, expr: FilterExpr) -> Result<CompiledFilter, WirerustError> {
        Ok(CompiledFilter::new(
            expr,
            Arc::clone(&self.schema),
            Arc::clone(&self.functions),
        ))
    }
    /// Parse and compile a filter expression string in one step.
    pub fn parse_and_compile(&self, expr: &str) -> Result<CompiledFilter, WirerustError> {
        let parsed = self.parse_filter(expr)?;
        self.compile_filter(parsed)
    }
    /// Execute a compiled filter against a context.
    pub fn execute(
        &self,
        filter: &CompiledFilter,
        ctx: &FilterContext,
    ) -> Result<bool, WirerustError> {
        filter.execute(ctx)
    }
}

/// Builder for WirerustEngine, for ergonomic embedding and configuration.
pub struct WirerustEngineBuilder {
    schema_builder: FilterSchemaBuilder,
    functions: FunctionRegistry,
    use_builtins: bool,
}

impl Default for WirerustEngineBuilder {
    fn default() -> Self {
        Self {
            schema_builder: FilterSchemaBuilder::new(),
            functions: FunctionRegistry::new(),
            use_builtins: true,
        }
    }
}

impl WirerustEngineBuilder {
    /// Create a new engine builder.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a field to the schema.
    pub fn field(mut self, name: impl Into<String>, ty: FieldType) -> Self {
        self.schema_builder = self.schema_builder.field(name, ty);
        self
    }
    /// Register a custom function.
    pub fn register_function<F: FilterFunction + 'static>(
        mut self,
        name: impl Into<String>,
        func: F,
    ) -> Self {
        self.functions.register(name, func);
        self
    }
    /// Disable built-in functions (by default, builtins are registered).
    pub fn no_builtins(mut self) -> Self {
        self.use_builtins = false;
        self
    }
    /// Build the engine.
    pub fn build(self) -> WirerustEngine {
        let schema = self.schema_builder.build();
        let mut functions = self.functions;
        if self.use_builtins {
            register_builtins(&mut functions);
        }
        WirerustEngine::with_functions(schema, functions)
    }
}

/// Example usage:
///
/// ```rust
/// use wirerust::{WirerustEngineBuilder, FieldType};
/// let engine = WirerustEngineBuilder::new()
///     .field("foo", FieldType::Int)
///     .field("bar", FieldType::Bytes)
///     .build();
/// ```
#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::FilterContextBuilder;
    use crate::schema::FilterSchemaBuilder;
    use crate::types::FieldType;

    #[test]
    fn test_wirerust_engine_end_to_end() {
        let schema = FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .build();
        let engine = WirerustEngine::new(schema);
        let filter = engine
            .parse_and_compile("foo == 42 && bar == \"baz\"")
            .unwrap();
        let ctx = FilterContextBuilder::new(&engine.schema)
            .set_int("foo", 42)
            .unwrap()
            .set_bytes("bar", b"baz")
            .unwrap()
            .build();
        let result = engine.execute(&filter, &ctx).unwrap();
        assert!(result);
    }

    #[test]
    fn test_engine_builder_minimal() {
        let engine = WirerustEngineBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .build();
        let filter = engine
            .parse_and_compile("foo == 1 && bar == \"abc\"")
            .unwrap();
        let ctx = FilterContextBuilder::new(&engine.schema)
            .set_int("foo", 1)
            .unwrap()
            .set_bytes("bar", b"abc")
            .unwrap()
            .build();
        assert!(engine.execute(&filter, &ctx).unwrap());
    }

    #[test]
    fn test_engine_builder_with_custom_function() {
        struct AlwaysTrue;
        impl FilterFunction for AlwaysTrue {
            fn call(&self, _args: &[LiteralValue]) -> Option<LiteralValue> {
                Some(LiteralValue::Bool(true))
            }
        }
        let engine = WirerustEngineBuilder::new()
            .field("foo", FieldType::Int)
            .register_function("always_true", AlwaysTrue)
            .build();
        let filter = engine
            .parse_and_compile("always_true() && foo == 5")
            .unwrap();
        let ctx = FilterContextBuilder::new(&engine.schema)
            .set_int("foo", 5)
            .unwrap()
            .build();
        assert!(engine.execute(&filter, &ctx).unwrap());
    }

    #[test]
    fn test_engine_builder_no_builtins() {
        let engine = WirerustEngineBuilder::new()
            .field("foo", FieldType::Int)
            .no_builtins()
            .build();
        // Built-in function 'len' should not be available
        let filter = engine.parse_and_compile("len(foo)").unwrap();
        let ctx = FilterContextBuilder::new(&engine.schema)
            .set_int("foo", 1)
            .unwrap()
            .build();
        let result = engine.execute(&filter, &ctx);
        assert!(
            result.is_err(),
            "Expected error when executing missing built-in function"
        );
    }
}
