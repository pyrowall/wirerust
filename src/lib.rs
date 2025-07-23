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

use thiserror::Error;
use std::sync::Arc;

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

pub struct WirerustEngine {
    pub schema: Arc<FilterSchema>,
    pub functions: Arc<FunctionRegistry>,
}

impl WirerustEngine {
    pub fn new(schema: FilterSchema) -> Self {
        let mut functions = FunctionRegistry::new();
        register_builtins(&mut functions);
        Self {
            schema: Arc::new(schema),
            functions: Arc::new(functions),
        }
    }
    pub fn with_functions(schema: FilterSchema, functions: FunctionRegistry) -> Self {
        Self {
            schema: Arc::new(schema),
            functions: Arc::new(functions),
        }
    }
    pub fn parse_filter(&self, expr: &str) -> Result<FilterExpr, WirerustError> {
        FilterParser::parse(expr, &self.schema)
    }
    pub fn compile_filter(&self, expr: FilterExpr) -> Result<CompiledFilter, WirerustError> {
        Ok(CompiledFilter::new(expr, Arc::clone(&self.schema), Arc::clone(&self.functions)))
    }
    pub fn parse_and_compile(&self, expr: &str) -> Result<CompiledFilter, WirerustError> {
        let parsed = self.parse_filter(expr)?;
        self.compile_filter(parsed)
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
            .set_int("foo", 42).unwrap()
            .set_bytes("bar", b"baz").unwrap()
            .build();
        let result = engine.execute(&filter, &ctx).unwrap();
        assert!(result);
    }
}
