//! Functions module: defines built-in and user-defined filter functions.
//!
//! This module provides traits and registries for filter functions.

use crate::types::LiteralValue;
use std::collections::HashMap;
use std::sync::Arc;

pub trait FilterFunction: Send + Sync {
    fn call(&self, args: &[LiteralValue]) -> Option<LiteralValue>;
}

pub struct FunctionRegistry {
    functions: HashMap<String, Arc<dyn FilterFunction>>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        Self { functions: HashMap::new() }
    }

    pub fn register<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: FilterFunction + 'static,
    {
        self.functions.insert(name.into(), Arc::new(func));
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn FilterFunction>> {
        self.functions.get(name)
    }
}

impl Clone for FunctionRegistry {
    fn clone(&self) -> Self {
        Self {
            functions: self.functions.clone(),
        }
    }
}

macro_rules! builtin_functions {
    ($( $name:ident: $func_name:expr, $args:ident => $body:block ),* $(,)?) => {
        $(
            pub struct $name;
            impl FilterFunction for $name {
                fn call(&self, $args: &[LiteralValue]) -> Option<LiteralValue> $body
            }
        )*
        pub fn register_builtins(reg: &mut FunctionRegistry) {
            $(reg.register($func_name, $name);)*
        }
    };
}

builtin_functions! {
    LenFunction: "len", args => {
        if let Some(LiteralValue::Array(arr)) = args.get(0) {
            Some(LiteralValue::Int(arr.len() as i64))
        } else {
            None
        }
    },
    UpperFunction: "upper", args => {
        if let Some(LiteralValue::Bytes(bytes)) = args.get(0) {
            let s = String::from_utf8_lossy(&bytes).to_uppercase();
            Some(LiteralValue::Bytes(s.into_bytes()))
        } else {
            None
        }
    },
    LowerFunction: "lower", args => {
        if let Some(LiteralValue::Bytes(bytes)) = args.get(0) {
            let s = String::from_utf8_lossy(&bytes).to_lowercase();
            Some(LiteralValue::Bytes(s.into_bytes()))
        } else {
            None
        }
    },
    SumFunction: "sum", args => {
        if let Some(LiteralValue::Array(arr)) = args.get(0) {
            let sum: i64 = arr.iter().filter_map(|v| if let LiteralValue::Int(i) = v { Some(*i) } else { None }).sum();
            Some(LiteralValue::Int(sum))
        } else {
            None
        }
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_register_and_call_len() {
        let mut reg = FunctionRegistry::new();
        reg.register("len", LenFunction);
        let arr = LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(2)]);
        let result = reg.get("len").unwrap().call(&[arr]);
        assert_eq!(result, Some(LiteralValue::Int(2)));
    }
    #[test]
    fn test_upper_function() {
        let mut reg = FunctionRegistry::new();
        reg.register("upper", UpperFunction);
        let val = LiteralValue::Bytes(b"hello".to_vec());
        let result = reg.get("upper").unwrap().call(&[val]);
        assert_eq!(result, Some(LiteralValue::Bytes(b"HELLO".to_vec())));
    }
    #[test]
    fn test_sum_function() {
        let mut reg = FunctionRegistry::new();
        reg.register("sum", SumFunction);
        let arr = LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(2), LiteralValue::Int(3)]);
        let result = reg.get("sum").unwrap().call(&[arr]);
        assert_eq!(result, Some(LiteralValue::Int(6)));
    }
} 