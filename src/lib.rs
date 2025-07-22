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

mod schema;
mod expr;
mod compiler;
mod filter;
mod context;
mod types;
mod functions;

pub use schema::*;
pub use expr::*;
pub use compiler::*;
pub use filter::*;
pub use context::*;
pub use types::*;
pub use functions::*;

use std::fmt;

/// Unified error type for Wirerust operations
#[derive(Debug)]
pub enum WirerustError {
    ParseError(String),
    TypeError(String),
    FunctionError(String),
    FieldNotFound(String),
    ExecutionError(String),
    Other(String),
}

impl fmt::Display for WirerustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WirerustError::ParseError(e) => write!(f, "Parse error: {}", e),
            WirerustError::TypeError(e) => write!(f, "Type error: {}", e),
            WirerustError::FunctionError(e) => write!(f, "Function error: {}", e),
            WirerustError::FieldNotFound(e) => write!(f, "Field not found: {}", e),
            WirerustError::ExecutionError(e) => write!(f, "Execution error: {}", e),
            WirerustError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl std::error::Error for WirerustError {}

pub struct WirerustEngine {
    pub schema: FilterSchema,
    pub functions: FunctionRegistry,
}

impl WirerustEngine {
    pub fn new(schema: FilterSchema) -> Self {
        let mut functions = FunctionRegistry::new();
        register_builtins(&mut functions);
        Self { schema, functions }
    }
    pub fn with_functions(schema: FilterSchema, functions: FunctionRegistry) -> Self {
        Self { schema, functions }
    }
    pub fn parse_filter(&self, expr: &str) -> Result<FilterExpr, WirerustError> {
        FilterParser::parse(expr, &self.schema)
    }
    pub fn compile_filter(&self, expr: FilterExpr) -> CompiledFilter {
        CompiledFilter::new(expr, self.schema.clone(), self.functions.clone())
    }
    pub fn parse_and_compile(&self, expr: &str) -> Result<CompiledFilter, WirerustError> {
        let parsed = self.parse_filter(expr)?;
        Ok(self.compile_filter(parsed))
    }
    pub fn execute(&self, filter: &CompiledFilter, ctx: &FilterContext) -> Result<bool, WirerustError> {
        filter.execute(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FieldType;
    use crate::schema::FilterSchemaBuilder;
    use crate::context::FilterContextBuilder;

    #[test]
    fn test_wirerust_engine_end_to_end() {
        let schema = FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .build();
        let engine = WirerustEngine::new(schema);
        let filter = engine.parse_and_compile("foo == 42 && bar == \"baz\"").unwrap();
        let ctx = FilterContextBuilder::new(&engine.schema)
            .set_int("foo", 42)
            .set_bytes("bar", b"baz")
            .build();
        let result = engine.execute(&filter, &ctx).unwrap();
        assert!(result);
    }
}
