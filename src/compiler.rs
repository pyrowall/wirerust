//! Compiler module: compiles AST into an intermediate representation (IR) for execution.
//!
//! This module provides traits and implementations for compiling filter expressions.

use crate::expr::{FilterExpr, LogicalOp, ComparisonOp};
use crate::context::FilterContext;
use crate::schema::FilterSchema;
use crate::types::LiteralValue;
use crate::functions::{FunctionRegistry, BuiltinFunctionId, call_builtin};
use crate::WirerustError;
use std::sync::Arc;
use crate::ir::{Instruction, IrStack};

/// A compiled filter in IR form.
pub struct IrCompiledFilter {
    pub bytecode: Vec<Instruction>,
    pub schema: Arc<FilterSchema>,
    pub functions: Arc<FunctionRegistry>,
}

impl IrCompiledFilter {
    /// Execute the IR filter against a context.
    pub fn execute(&self, ctx: &FilterContext) -> Result<bool, WirerustError> {
        let mut stack: IrStack = Vec::with_capacity(16);
        let bytecode = &self.bytecode;
        let mut pc = 0;
        while pc < bytecode.len() {
            match &bytecode[pc] {
                Instruction::LoadField(fid) => {
                    let val = ctx.get_by_id(*fid).cloned().unwrap_or(LiteralValue::Bool(false));
                    stack.push(val);
                }
                Instruction::LoadLiteral(lit) => {
                    stack.push(lit.clone());
                }
                Instruction::CallFunction(fid, argc) => {
                    let argc = *argc as usize;
                    let args: Vec<_> = stack.split_off(stack.len() - argc);
                    // Fast-path for built-in functions
                    if let Some(name) = self.functions.function_name(*fid) {
                        if let Some(builtin_id) = BuiltinFunctionId::from_name(name) {
                            let result = call_builtin(builtin_id, &args).ok_or_else(|| WirerustError::FunctionError(format!("Builtin function call failed for {name}")))?;
                            stack.push(result);
                            pc += 1;
                            continue;
                        }
                    }
                    let func = self.functions.get_by_id(*fid).ok_or_else(|| WirerustError::FunctionError(format!("Function ID {fid} not found")))?;
                    let result = func.call(&args).ok_or_else(|| WirerustError::FunctionError(format!("Function call failed for ID {fid}")))?;
                    stack.push(result);
                }
                Instruction::CompareEq => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(left == right));
                }
                Instruction::CompareNeq => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(left != right));
                }
                Instruction::CompareLt => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(cmp_ord(&left, &right, |a, b| a < b)));
                }
                Instruction::CompareLte => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(cmp_ord(&left, &right, |a, b| a <= b)));
                }
                Instruction::CompareGt => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(cmp_ord(&left, &right, |a, b| a > b)));
                }
                Instruction::CompareGte => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(cmp_ord(&left, &right, |a, b| a >= b)));
                }
                Instruction::CompareIn => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(cmp_in(&left, &right)));
                }
                Instruction::CompareNotIn => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(!cmp_in(&left, &right)));
                }
                Instruction::CompareMatches => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(cmp_matches(&left, &right)));
                }
                Instruction::CompareWildcard { strict } => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(cmp_wildcard(&left, &right, *strict)));
                }
                Instruction::CompareContains => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(cmp_contains(&left, &right)));
                }
                Instruction::LogicalAnd => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    // Short-circuit: if left is false, skip right
                    if !to_bool(&left) {
                        stack.push(LiteralValue::Bool(false));
                        // skip right (already popped)
                    } else {
                        stack.push(LiteralValue::Bool(to_bool(&right)));
                    }
                }
                Instruction::LogicalOr => {
                    let right = stack.pop().unwrap();
                    let left = stack.pop().unwrap();
                    // Short-circuit: if left is true, skip right
                    if to_bool(&left) {
                        stack.push(LiteralValue::Bool(true));
                        // skip right (already popped)
                    } else {
                        stack.push(LiteralValue::Bool(to_bool(&right)));
                    }
                }
                Instruction::LogicalNot => {
                    let a = stack.pop().unwrap();
                    stack.push(LiteralValue::Bool(!to_bool(&a)));
                }
            }
            pc += 1;
        }
        match stack.pop() {
            Some(LiteralValue::Bool(b)) => Ok(b),
            Some(other) => Ok(to_bool(&other)),
            None => Err(WirerustError::ExecutionError("Empty stack after execution".into())),
        }
    }
}

fn to_bool(val: &LiteralValue) -> bool {
    match val {
        LiteralValue::Bool(b) => *b,
        LiteralValue::Int(i) => *i != 0,
        LiteralValue::Bytes(_) => true,
        LiteralValue::Array(arr) => !arr.is_empty(),
        LiteralValue::Ip(_) => true,
        LiteralValue::Map(map) => !map.is_empty(),
    }
}

pub struct DefaultCompiler;

impl DefaultCompiler {
    /// Compile a filter expression into IR bytecode.
    pub fn compile_ir(expr: &FilterExpr, schema: &FilterSchema, functions: &FunctionRegistry, code: &mut Vec<Instruction>) {
        match expr {
            FilterExpr::LogicalOp { op, left, right } => {
                Self::compile_ir(left, schema, functions, code);
                Self::compile_ir(right, schema, functions, code);
                match op {
                    LogicalOp::And => code.push(Instruction::LogicalAnd),
                    LogicalOp::Or => code.push(Instruction::LogicalOr),
                }
            }
            FilterExpr::Comparison { left, op, right } => {
                Self::compile_ir(left, schema, functions, code);
                Self::compile_ir(right, schema, functions, code);
                match op {
                    ComparisonOp::Eq => code.push(Instruction::CompareEq),
                    ComparisonOp::Neq => code.push(Instruction::CompareNeq),
                    ComparisonOp::Lt => code.push(Instruction::CompareLt),
                    ComparisonOp::Lte => code.push(Instruction::CompareLte),
                    ComparisonOp::Gt => code.push(Instruction::CompareGt),
                    ComparisonOp::Gte => code.push(Instruction::CompareGte),
                    ComparisonOp::In => code.push(Instruction::CompareIn),
                    ComparisonOp::NotIn => code.push(Instruction::CompareNotIn),
                    ComparisonOp::Matches => code.push(Instruction::CompareMatches),
                    ComparisonOp::Wildcard => code.push(Instruction::CompareWildcard { strict: false }),
                    ComparisonOp::StrictWildcard => code.push(Instruction::CompareWildcard { strict: true }),
                    ComparisonOp::Contains => code.push(Instruction::CompareContains),
                }
            }
            FilterExpr::Not(inner) => {
                Self::compile_ir(inner, schema, functions, code);
                code.push(Instruction::LogicalNot);
            }
            FilterExpr::Value(val) => {
                // If this is a field reference (Bytes, and field exists in schema), emit LoadField; else, LoadLiteral
                if let LiteralValue::Bytes(bytes) = val {
                    if let Ok(field) = std::str::from_utf8(bytes) {
                        if let Some(fid) = schema.field_id(field) {
                            code.push(Instruction::LoadField(fid));
                            return;
                        }
                    }
                }
                code.push(Instruction::LoadLiteral(val.clone()));
            }
            FilterExpr::FunctionCall { name, args } => {
                for arg in args {
                    Self::compile_ir(arg, schema, functions, code);
                }
                if let Some(fid) = functions.function_id(name) {
                    code.push(Instruction::CallFunction(fid, args.len() as u8));
                } else {
                    // Unknown function: error at runtime
                    code.push(Instruction::CallFunction(usize::MAX, args.len() as u8));
                }
            }
            FilterExpr::List(vals) => {
                code.push(Instruction::LoadLiteral(LiteralValue::Array(Arc::new(vals.clone()))));
            }
        }
    }

    pub fn compile(expr: FilterExpr, schema: Arc<FilterSchema>, functions: Arc<FunctionRegistry>) -> IrCompiledFilter {
        let mut bytecode: Vec<Instruction> = Vec::new();
        Self::compile_ir(&expr, &schema, &functions, &mut bytecode);
        IrCompiledFilter {
            bytecode,
            schema: Arc::clone(&schema),
            functions: Arc::clone(&functions),
        }
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
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"baz".to_vec())), &sch).unwrap();
        ctx.set("arr", LiteralValue::Array(Arc::new(vec![LiteralValue::Int(1), LiteralValue::Int(2)])), &sch).unwrap();
        ctx
    }

    #[test]
    fn test_compile_and_execute_comparison_eq() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"foo".to_vec())))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(42))),
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_logical_and_or() {
        let expr = FilterExpr::LogicalOp {
            op: LogicalOp::And,
            left: Box::new(FilterExpr::Comparison {
                left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"foo".to_vec())))),
                op: ComparisonOp::Eq,
                right: Box::new(FilterExpr::Value(LiteralValue::Int(42))),
            }),
            right: Box::new(FilterExpr::Comparison {
                left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"bar".to_vec())))),
                op: ComparisonOp::Eq,
                right: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"baz".to_vec())))),
            }),
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_not() {
        let expr = FilterExpr::Not(Box::new(FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"foo".to_vec())))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(0))),
        }));
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_in() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"foo".to_vec())))),
            op: ComparisonOp::In,
            right: Box::new(FilterExpr::Value(LiteralValue::Array(Arc::new(vec![LiteralValue::Int(1), LiteralValue::Int(42)])))),
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_function_call() {
        let mut functions = FunctionRegistry::new();
        functions.register("len", LenFunction);
        let expr = FilterExpr::FunctionCall {
            name: "len".to_string(),
            args: vec![FilterExpr::Value(LiteralValue::Array(Arc::new(vec![LiteralValue::Int(1), LiteralValue::Int(2)])))],
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(functions));
        // len([1,2]) returns Int(2), which converts to true via to_bool since 2 != 0
        assert!(filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_unknown_field() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"unknown".to_vec())))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Int(1))),
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(!filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_wrong_type() {
        let expr = FilterExpr::Comparison {
            left: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"foo".to_vec())))),
            op: ComparisonOp::Eq,
            right: Box::new(FilterExpr::Value(LiteralValue::Bytes(Arc::new(b"not an int".to_vec())))),
        };
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        assert!(!filter.execute(&context()).unwrap());
    }

    #[test]
    fn test_compile_and_execute_contains_string() {
        let expr = FilterParser::parse("bar contains \"oba\"", &schema()).unwrap();
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let mut ctx = context();
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"foobar".to_vec())), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"baz".to_vec())), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"oba".to_vec())), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"".to_vec())), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"foobaroba".to_vec())), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
    }

    #[test]
    fn test_compile_and_execute_contains_array() {
        let expr = FilterParser::parse("arr contains 2", &schema()).unwrap();
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let mut ctx = context();
        ctx.set("arr", LiteralValue::Array(Arc::new(vec![
            LiteralValue::Int(1),
            LiteralValue::Int(2),
            LiteralValue::Int(3),
        ])), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
        ctx.set("arr", LiteralValue::Array(Arc::new(vec![
            LiteralValue::Int(4),
            LiteralValue::Int(5),
        ])), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap());
        ctx.set("arr", LiteralValue::Array(Arc::new(vec![])), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap());
        ctx.set("arr", LiteralValue::Array(Arc::new(vec![
            LiteralValue::Int(2),
        ])), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
    }

    #[test]
    fn test_compile_and_execute_wildcard() {
        let expr = FilterParser::parse("bar wildcard \"b*r\"", &schema()).unwrap();
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let mut ctx = context();
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"bar".to_vec())), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"BAR".to_vec())), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap()); // case-insensitive
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"bxxr".to_vec())), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"bxxz".to_vec())), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"foo".to_vec())), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap());
    }

    #[test]
    fn test_compile_and_execute_strict_wildcard() {
        let expr = FilterParser::parse("bar strict wildcard \"b*r\"", &schema()).unwrap();
        let filter = DefaultCompiler::compile(expr, Arc::new(schema()), Arc::new(FunctionRegistry::new()));
        let mut ctx = context();
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"bar".to_vec())), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"BAR".to_vec())), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap()); // case-sensitive
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"bxxr".to_vec())), &schema()).unwrap();
        assert!(filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"bxxz".to_vec())), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap());
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"foo".to_vec())), &schema()).unwrap();
        assert!(!filter.execute(&ctx).unwrap());
    }
} 