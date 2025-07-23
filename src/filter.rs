//! Filter module: wraps compiled filters and provides the main execution API.
//!
//! This module provides the CompiledFilter type.

use crate::compiler::IrCompiledFilter;
use crate::schema::FilterSchema;
use crate::context::FilterContext;
use crate::WirerustError;
use std::sync::Arc;

/// A compiled filter, ready for execution.
pub struct CompiledFilter {
    ir: IrCompiledFilter,
}

impl CompiledFilter {
    /// Create a new compiled filter from an expression, schema, and function registry.
    pub fn new(expr: crate::expr::FilterExpr, schema: std::sync::Arc<crate::schema::FilterSchema>, functions: std::sync::Arc<crate::functions::FunctionRegistry>) -> Self {
        let ir = crate::compiler::DefaultCompiler::compile(expr, schema, functions);
        Self { ir }
    }
    /// Execute the filter against a context.
    pub fn execute(&self, context: &crate::context::FilterContext) -> Result<bool, crate::WirerustError> {
        self.ir.execute(context)
    }
    /// Get a reference to the schema used by this filter.
    pub fn schema(&self) -> &crate::schema::FilterSchema {
        &self.ir.schema
    }
    /// Get a reference to the function registry used by this filter.
    pub fn functions(&self) -> &crate::functions::FunctionRegistry {
        &self.ir.functions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldType, LiteralValue};
    use crate::schema::FilterSchemaBuilder;
    use crate::context::FilterContext;
    use crate::expr::{FilterExpr, ComparisonOp};
    use crate::functions::FunctionRegistry;

    fn schema() -> FilterSchema {
        FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .build()
    }

    fn context() -> FilterContext {
        let mut ctx = FilterContext::new();
        let sch = schema();
        ctx.set("foo", LiteralValue::Int(42), &sch).unwrap();
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"baz".to_vec()).into()), &sch).unwrap();
        ctx
    }

    #[test]
    fn test_compiled_filter_execute_true() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"foo".to_vec()).into()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(42))),
        };
        let filter = CompiledFilter::new(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compiled_filter_execute_false() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"foo".to_vec()).into()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(0))),
        };
        let filter = CompiledFilter::new(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(!filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compiled_filter_schema_access() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"foo".to_vec()).into()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(42))),
        };
        let filter = CompiledFilter::new(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let sch = filter.schema();
        assert_eq!(sch.get_field_type("foo"), Some(&FieldType::Int));
    }
} 