//! Compiler module: compiles AST into an intermediate representation (IR) for execution.
//!
//! This module provides traits and implementations for compiling filter expressions.

use crate::expr::{FilterExpr, LogicalOp, ComparisonOp};
use crate::context::FilterContext;
use crate::schema::FilterSchema;
use crate::types::LiteralValue;
use crate::functions::FunctionRegistry;

pub struct DefaultCompiler;

impl DefaultCompiler {
    pub fn compile(expr: FilterExpr, schema: FilterSchema, functions: FunctionRegistry) -> Box<dyn Fn(&FilterContext) -> bool + Send + Sync + 'static> {
        match expr {
            FilterExpr::LogicalOp { op, left, right } => {
                let l = DefaultCompiler::compile(*left.clone(), schema.clone(), functions.clone());
                let r = DefaultCompiler::compile(*right.clone(), schema.clone(), functions.clone());
                match op {
                    LogicalOp::And => Box::new(move |ctx| l(ctx) && r(ctx)),
                    LogicalOp::Or => Box::new(move |ctx| l(ctx) || r(ctx)),
                }
            }
            FilterExpr::Comparison { left, op, right } => {
                let left = *left;
                let right = *right;
                let functions = functions.clone();
                Box::new(move |ctx| {
                    let lval = eval_expr(&left, ctx, &functions);
                    let rval = eval_expr(&right, ctx, &functions);
                    match op {
                        ComparisonOp::Eq => lval == rval,
                        ComparisonOp::Neq => lval != rval,
                        ComparisonOp::Lt => cmp_ord(&lval, &rval, |a, b| a < b),
                        ComparisonOp::Lte => cmp_ord(&lval, &rval, |a, b| a <= b),
                        ComparisonOp::Gt => cmp_ord(&lval, &rval, |a, b| a > b),
                        ComparisonOp::Gte => cmp_ord(&lval, &rval, |a, b| a >= b),
                        ComparisonOp::In => cmp_in(&lval, &rval),
                        ComparisonOp::NotIn => !cmp_in(&lval, &rval),
                        ComparisonOp::Matches => cmp_matches(&lval, &rval),
                    }
                })
            }
            FilterExpr::Not(inner) => {
                let inner_fn = DefaultCompiler::compile(*inner.clone(), schema.clone(), functions.clone());
                Box::new(move |ctx| !inner_fn(ctx))
            }
            FilterExpr::Value(val) => {
                let val = val.clone();
                let functions = functions.clone();
                Box::new(move |ctx| {
                    let result = eval_expr(&FilterExpr::Value(val.clone()), ctx, &functions);
                    // Convert the result to boolean
                    match result {
                        LiteralValue::Bool(b) => b,
                        LiteralValue::Int(i) => i != 0,
                        LiteralValue::Bytes(_) => true, // Non-empty string/bytes are truthy
                        LiteralValue::Array(arr) => !arr.is_empty(), // Non-empty arrays are truthy
                        LiteralValue::Ip(_) => true, // IP addresses are truthy
                        LiteralValue::Map(map) => !map.is_empty(), // Non-empty maps are truthy
                    }
                })
            }
            FilterExpr::FunctionCall { name, args } => {
                let name = name.clone();
                let arg_exprs = args.clone();
                let functions = functions.clone();
                Box::new(move |ctx| {
                    let func = functions.get(&name);
                    if let Some(func) = func {
                        let arg_vals: Vec<_> = arg_exprs.iter().map(|e| eval_expr(e, ctx, &functions)).collect();
                        let result = func.call(&arg_vals).unwrap_or(LiteralValue::Bool(false));
                        matches!(result, LiteralValue::Bool(true))
                    } else {
                        false
                    }
                })
            }
            FilterExpr::List(_) => Box::new(|_| false), // Not meaningful at top level
        }
    }
}

fn eval_expr(expr: &FilterExpr, ctx: &FilterContext, functions: &FunctionRegistry) -> LiteralValue {
    match expr {
        FilterExpr::Value(val) => {
            if let LiteralValue::Bytes(bytes) = val {
                if let Ok(field) = std::str::from_utf8(bytes) {
                    if let Some(v) = ctx.get(field) {
                        return v.clone();
                    }
                }
            }
            val.clone()
        }
        FilterExpr::FunctionCall { name, args } => {
            let func = functions.get(name);
            let arg_vals: Vec<_> = args.iter().map(|e| eval_expr(e, ctx, functions)).collect();
            if let Some(func) = func {
                func.call(&arg_vals).unwrap_or(LiteralValue::Bool(false))
            } else {
                LiteralValue::Bool(false)
            }
        }
        FilterExpr::Comparison { .. } => LiteralValue::Bool(false),
        FilterExpr::LogicalOp { .. } => LiteralValue::Bool(false),
        FilterExpr::Not(_) => LiteralValue::Bool(false),
        FilterExpr::List(vals) => LiteralValue::Array(vals.clone()),
    }
}

// Helper for ordered comparisons
fn cmp_ord<F>(a: &LiteralValue, b: &LiteralValue, cmp: F) -> bool
where
    F: Fn(&i64, &i64) -> bool,
{
    match (a, b) {
        (LiteralValue::Int(a), LiteralValue::Int(b)) => cmp(a, b),
        // TODO: Add more type support (e.g., Bytes, Ip)
        _ => false,
    }
}

// Helper for 'in' and 'not in' comparisons
fn cmp_in(a: &LiteralValue, b: &LiteralValue) -> bool {
    match b {
        LiteralValue::Array(arr) => arr.contains(a),
        _ => false,
    }
}

// Helper for 'matches' (regex) comparisons
fn cmp_matches(a: &LiteralValue, b: &LiteralValue) -> bool {
    match (a, b) {
        (LiteralValue::Bytes(bytes), LiteralValue::Bytes(pattern)) => {
            if let (Ok(s), Ok(pat)) = (std::str::from_utf8(bytes), std::str::from_utf8(pattern)) {
                // Use regex crate if available, else fallback to substring
                #[cfg(feature = "regex")] {
                    if let Ok(re) = regex::Regex::new(pat) {
                        re.is_match(s)
                    } else {
                        false
                    }
                }
                #[cfg(not(feature = "regex"))]
                {
                    s.contains(pat)
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldType, LiteralValue};
    use crate::schema::FilterSchemaBuilder;
    use crate::context::FilterContext;
    use crate::expr::{FilterExpr, ComparisonOp, LogicalOp};
    use crate::functions::{FunctionRegistry, LenFunction};

    fn schema() -> FilterSchema {
        FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .field("arr", FieldType::Array(Box::new(FieldType::Int)))
            .build()
    }

    fn context() -> FilterContext {
        let mut ctx = FilterContext::new();
        let sch = schema();
        ctx.set("foo", LiteralValue::Int(42), &sch).unwrap();
        ctx.set("bar", LiteralValue::Bytes(b"baz".to_vec()), &sch).unwrap();
        ctx.set("arr", LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(2)]), &sch).unwrap();
        ctx
    }

    #[test]
    fn test_compile_and_execute_comparison_eq() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(42))),
        };
        let filter = DefaultCompiler::compile(expr, schema(), FunctionRegistry::new());
        assert!(filter(&context()));
    }

    #[test]
    fn test_compile_and_execute_logical_and_or() {
        let expr = FilterExpr::LogicalOp {
            op: LogicalOp::And,
            left: Box::new(FilterExpr::Comparison {
                left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
                op: ComparisonOp::Eq,
                right: Box::new(FilterExpr::Value(LiteralValue::Int(42))),
            }),
            right: Box::new(FilterExpr::Comparison {
                left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"bar".to_vec()))),
                op: ComparisonOp::Eq,
                right: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"baz".to_vec()))),
            }),
        };
        let filter = DefaultCompiler::compile(expr, schema(), FunctionRegistry::new());
        assert!(filter(&context()));
    }

    #[test]
    fn test_compile_and_execute_not() {
        let expr = FilterExpr::Not(Box::new(FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(0))),
        }));
        let filter = DefaultCompiler::compile(expr, schema(), FunctionRegistry::new());
        assert!(filter(&context()));
    }

    #[test]
    fn test_compile_and_execute_in() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::In,
            right: Box::new(FilterExpr::Value(LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(42)]))),
        };
        let filter = DefaultCompiler::compile(expr, schema(), FunctionRegistry::new());
        assert!(filter(&context()));
    }

    #[test]
    fn test_compile_and_execute_function_call() {
        let mut functions = FunctionRegistry::new();
        functions.register("len", LenFunction);
        let expr = FilterExpr::FunctionCall {
            name: "len".to_string(),
            args: vec![FilterExpr::Value(LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(2)]))],
        };
        let filter = DefaultCompiler::compile(expr, schema(), functions);
        // len([1,2]) returns Int(2), but top-level expects Bool, so should be false
        assert!(!filter(&context()));
    }

    #[test]
    fn test_compile_and_execute_unknown_field() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"unknown".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(1))),
        };
        let filter = DefaultCompiler::compile(expr, schema(), FunctionRegistry::new());
        assert!(!filter(&context()));
    }

    #[test]
    fn test_compile_and_execute_wrong_type() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"not an int".to_vec()))),
        };
        let filter = DefaultCompiler::compile(expr, schema(), FunctionRegistry::new());
        assert!(!filter(&context()));
    }
} 