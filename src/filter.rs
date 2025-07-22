//! Filter module: wraps compiled filters and provides the main execution API.
//!
//! This module provides the CompiledFilter type.

use crate::compiler::DefaultCompiler;
use crate::expr::FilterExpr;
use crate::context::FilterContext;
use crate::schema::FilterSchema;
use crate::functions::FunctionRegistry;
use crate::WirerustError;

pub struct CompiledFilter {
    schema: FilterSchema,
    exec: Box<dyn Fn(&FilterContext) -> Result<bool, WirerustError> + Send + Sync>,
}

impl CompiledFilter {
    pub fn new(expr: FilterExpr, schema: FilterSchema, functions: FunctionRegistry) -> Self {
        let exec = DefaultCompiler::compile(expr, schema.clone(), functions.clone());
        Self { schema, exec }
    }

    pub fn execute(&self, context: &FilterContext) -> Result<bool, WirerustError> {
        (self.exec)(context)
    }

    pub fn schema(&self) -> &FilterSchema {
        &self.schema
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
        ctx.set("bar", LiteralValue::Bytes(b"baz".to_vec()), &sch).unwrap();
        ctx
    }

    #[test]
    fn test_compiled_filter_execute_true() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(42))),
        };
        let filter = CompiledFilter::new(expr, schema(), FunctionRegistry::new());
        assert!(filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compiled_filter_execute_false() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(0))),
        };
        let filter = CompiledFilter::new(expr, schema(), FunctionRegistry::new());
        assert!(!filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compiled_filter_schema_access() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(42))),
        };
        let filter = CompiledFilter::new(expr, schema(), FunctionRegistry::new());
        let sch = filter.schema();
        assert_eq!(sch.get_field_type("foo"), Some(&FieldType::Int));
    }
} 