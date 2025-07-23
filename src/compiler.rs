//! Compiler module: compiles AST into an intermediate representation (IR) for execution.
//!
//! This module provides traits and implementations for compiling filter expressions.

use crate::expr::{FilterExpr, LogicalOp, ComparisonOp};
use crate::context::FilterContext;
use crate::schema::FilterSchema;
use crate::types::LiteralValue;
use crate::functions::FunctionRegistry;
use crate::WirerustError;
use std::sync::Arc;

pub struct DefaultCompiler;

impl DefaultCompiler {
    pub fn compile(expr: FilterExpr, schema: Arc<FilterSchema>, functions: Arc<FunctionRegistry>) -> Box<dyn Fn(&FilterContext) -> Result<bool, WirerustError> + Send + Sync + 'static> {
        match expr {
            FilterExpr::LogicalOp { op, left, right } => {
                let l = DefaultCompiler::compile(*left.clone(), Arc::clone(&schema), Arc::clone(&functions));
                let r = DefaultCompiler::compile(*right.clone(), Arc::clone(&schema), Arc::clone(&functions));
                match op {
                    LogicalOp::And => Box::new(move |ctx| Ok(l(ctx)? && r(ctx)?)),
                    LogicalOp::Or => Box::new(move |ctx| Ok(l(ctx)? || r(ctx)?)),
                }
            }
            FilterExpr::Comparison { left, op, right } => {
                let left = *left;
                let right = *right;
                let functions = Arc::clone(&functions);
                Box::new(move |ctx| {
                    let lval = eval_expr(&left, ctx, &functions);
                    let rval = eval_expr(&right, ctx, &functions);
                    let result = match op {
                        ComparisonOp::Eq => lval == rval,
                        ComparisonOp::Neq => lval != rval,
                        ComparisonOp::Lt => cmp_ord(&lval, &rval, |a, b| a < b),
                        ComparisonOp::Lte => cmp_ord(&lval, &rval, |a, b| a <= b),
                        ComparisonOp::Gt => cmp_ord(&lval, &rval, |a, b| a > b),
                        ComparisonOp::Gte => cmp_ord(&lval, &rval, |a, b| a >= b),
                        ComparisonOp::In => cmp_in(&lval, &rval),
                        ComparisonOp::NotIn => !cmp_in(&lval, &rval),
                        ComparisonOp::Matches => cmp_matches(&lval, &rval),
                        ComparisonOp::Wildcard => cmp_wildcard(&lval, &rval, false),
                        ComparisonOp::StrictWildcard => cmp_wildcard(&lval, &rval, true),
                        ComparisonOp::Contains => cmp_contains(&lval, &rval),
                    };
                    Ok(result)
                })
            }
            FilterExpr::Not(inner) => {
                let inner_fn = DefaultCompiler::compile(*inner.clone(), Arc::clone(&schema), Arc::clone(&functions));
                Box::new(move |ctx| Ok(!inner_fn(ctx)?))
            }
            FilterExpr::Value(val) => {
                let val = val.clone();
                let functions = Arc::clone(&functions);
                Box::new(move |ctx| {
                    let result = eval_expr(&FilterExpr::Value(val.clone()), ctx, &functions);
                    let b = match result {
                        LiteralValue::Bool(b) => b,
                        LiteralValue::Int(i) => i != 0,
                        LiteralValue::Bytes(_) => true,
                        LiteralValue::Array(arr) => !arr.is_empty(),
                        LiteralValue::Ip(_) => true,
                        LiteralValue::Map(map) => !map.is_empty(),
                    };
                    Ok(b)
                })
            }
            FilterExpr::FunctionCall { name, args } => {
                let name = name.clone();
                let arg_exprs = args.clone();
                let functions = Arc::clone(&functions);
                Box::new(move |ctx| {
                    let func = functions.get(&name);
                    if let Some(func) = func {
                        let arg_vals: Vec<_> = arg_exprs.iter().map(|e| eval_expr(e, ctx, &functions)).collect();
                        let result = func.call(&arg_vals).unwrap_or(LiteralValue::Bool(false));
                        Ok(matches!(result, LiteralValue::Bool(true)))
                    } else {
                        Err(WirerustError::FunctionError(format!("Function '{}' not found", name)))
                    }
                })
            }
            FilterExpr::List(_) => Box::new(|_| Ok(false)),
        }
    }
}

fn eval_expr(expr: &FilterExpr, ctx: &FilterContext, functions: &Arc<FunctionRegistry>) -> LiteralValue {
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

// Helper for wildcard and strict wildcard comparisons
fn cmp_wildcard(a: &LiteralValue, b: &LiteralValue, case_sensitive: bool) -> bool {
    match (a, b) {
        (LiteralValue::Bytes(bytes), LiteralValue::Bytes(pattern)) => {
            let s = match std::str::from_utf8(bytes) {
                Ok(s) => s,
                Err(_) => return false,
            };
            let pat = match std::str::from_utf8(pattern) {
                Ok(p) => p,
                Err(_) => return false,
            };
            wildcard_match(s, pat, case_sensitive)
        }
        _ => false,
    }
}

fn wildcard_match(s: &str, pat: &str, case_sensitive: bool) -> bool {
    let (s, pat) = if case_sensitive {
        (s.to_string(), pat.to_string())
    } else {
        (s.to_lowercase(), pat.to_lowercase())
    };
    wildcard_match_inner(&s, &pat)
}

fn wildcard_match_inner(s: &str, pat: &str) -> bool {
    let s_bytes = s.as_bytes();
    let pat_bytes = pat.as_bytes();
    wildcard_match_bytes(s_bytes, pat_bytes)
}

fn wildcard_match_bytes(s: &[u8], pat: &[u8]) -> bool {
    if pat.is_empty() {
        return s.is_empty();
    }
    if pat[0] == b'*' {
        for i in 0..=s.len() {
            if wildcard_match_bytes(&s[i..], &pat[1..]) {
                return true;
            }
        }
        false
    } else if !s.is_empty() && (pat[0] == s[0]) {
        wildcard_match_bytes(&s[1..], &pat[1..])
    } else {
        false
    }
}

// Helper for contains comparison
fn cmp_contains(a: &LiteralValue, b: &LiteralValue) -> bool {
    match (a, b) {
        (LiteralValue::Bytes(haystack), LiteralValue::Bytes(needle)) => {
            if let (Ok(h), Ok(n)) = (std::str::from_utf8(haystack), std::str::from_utf8(needle)) {
                h.contains(n)
            } else {
                false
            }
        }
        (LiteralValue::Array(arr), val) => arr.contains(val),
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
    use crate::expr::FilterParser;

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
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter(&context()).unwrap());
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
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_not() {
        let expr = FilterExpr::Not(Box::new(FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(0))),
        }));
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_in() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::In,
            right: Box::new(FilterExpr::Value(LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(42)]))),
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_function_call() {
        let mut functions = FunctionRegistry::new();
        functions.register("len", LenFunction);
        let expr = FilterExpr::FunctionCall {
            name: "len".to_string(),
            args: vec![FilterExpr::Value(LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(2)]))],
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(functions));
        // len([1,2]) returns Int(2), but top-level expects Bool, so should be false
        assert!(!filter(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_unknown_field() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"unknown".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(1))),
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(!filter(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_wrong_type() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec()))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Bytes(b"not an int".to_vec()))),
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(!filter(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_contains_string() {
        let expr = FilterParser::parse("bar contains \"oba\"", &schema()).unwrap();
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let mut ctx = context();
        ctx.set("bar", LiteralValue::Bytes(b"foobar".to_vec()), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"baz".to_vec()), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"oba".to_vec()), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"".to_vec()), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"foobaroba".to_vec()), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
    }

    #[test]
    fn test_compile_and_execute_contains_array() {
        let expr = FilterParser::parse("arr contains 2", &schema()).unwrap();
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let mut ctx = context();
        ctx.set("arr", LiteralValue::Array(vec![
            LiteralValue::Int(1),
            LiteralValue::Int(2),
            LiteralValue::Int(3),
        ]), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
        ctx.set("arr", LiteralValue::Array(vec![
            LiteralValue::Int(4),
            LiteralValue::Int(5),
        ]), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap());
        ctx.set("arr", LiteralValue::Array(vec![]), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap());
        ctx.set("arr", LiteralValue::Array(vec![
            LiteralValue::Int(2),
        ]), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
    }

    #[test]
    fn test_compile_and_execute_wildcard() {
        let expr = FilterParser::parse("bar wildcard \"b*r\"", &schema()).unwrap();
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let mut ctx = context();
        ctx.set("bar", LiteralValue::Bytes(b"bar".to_vec()), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"BAR".to_vec()), &schema()).unwrap();
        assert!(filter(&ctx).unwrap()); // case-insensitive
        ctx.set("bar", LiteralValue::Bytes(b"bxxr".to_vec()), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"bxxz".to_vec()), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"foo".to_vec()), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap());
    }

    #[test]
    fn test_compile_and_execute_strict_wildcard() {
        let expr = FilterParser::parse("bar strict wildcard \"b*r\"", &schema()).unwrap();
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let mut ctx = context();
        ctx.set("bar", LiteralValue::Bytes(b"bar".to_vec()), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"BAR".to_vec()), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap()); // case-sensitive
        ctx.set("bar", LiteralValue::Bytes(b"bxxr".to_vec()), &schema()).unwrap();
        assert!(filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"bxxz".to_vec()), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(b"foo".to_vec()), &schema()).unwrap();
        assert!(!filter(&ctx).unwrap());
    }
} 