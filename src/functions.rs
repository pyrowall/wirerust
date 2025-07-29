//! Functions module: defines built-in and user-defined filter functions.
//!
//! This module provides traits and registries for filter functions.

use crate::types::LiteralValue;
use std::collections::HashMap;
use std::sync::Arc;

pub trait FilterFunction: Send + Sync {
    fn call(&self, args: &[LiteralValue]) -> Option<LiteralValue>;
}

#[derive(Default)]
pub struct FunctionRegistry {
    functions: HashMap<String, Arc<dyn FilterFunction>>,
    function_names: Vec<String>, // index = FunctionId
    function_ids: HashMap<String, usize>, // name -> id
}

impl FunctionRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    /// Register a function and assign it a unique ID if not already present.
    pub fn register<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: FilterFunction + 'static,
    {
        let name = name.into();
        if !self.function_ids.contains_key(&name) {
            self.function_ids.insert(name.clone(), self.function_names.len());
            self.function_names.push(name.clone());
        }
        self.functions.insert(name, Arc::new(func));
    }
    /// Register a closure as a filter function.
    pub fn register_fn<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(&[LiteralValue]) -> Option<LiteralValue> + Send + Sync + 'static,
    {
        struct ClosureFn<F>(F);
        impl<F> FilterFunction for ClosureFn<F>
        where
            F: Fn(&[LiteralValue]) -> Option<LiteralValue> + Send + Sync + 'static,
        {
            fn call(&self, args: &[LiteralValue]) -> Option<LiteralValue> {
                (self.0)(args)
            }
        }
        self.register(name, ClosureFn(func));
    }
    /// Get the function ID for a given function name, if it exists.
    pub fn function_id(&self, name: &str) -> Option<usize> {
        self.function_ids.get(name).copied()
    }
    /// Get the function name for a given function ID, if it exists.
    pub fn function_name(&self, id: usize) -> Option<&str> {
        self.function_names.get(id).map(|s| s.as_str())
    }
    /// Get the total number of functions.
    pub fn num_functions(&self) -> usize {
        self.function_names.len()
    }
    pub fn get(&self, name: &str) -> Option<&Arc<dyn FilterFunction>> {
        self.functions.get(name)
    }
    /// Get a function by ID.
    pub fn get_by_id(&self, id: usize) -> Option<&Arc<dyn FilterFunction>> {
        self.function_names.get(id).and_then(|name| self.functions.get(name))
    }
}

impl Clone for FunctionRegistry {
    fn clone(&self) -> Self {
        Self {
            functions: self.functions.clone(),
            function_names: self.function_names.clone(),
            function_ids: self.function_ids.clone(),
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
        if let Some(LiteralValue::Array(arr)) = args.first() {
            Some(LiteralValue::Int(arr.len() as i64))
        } else {
            None
        }
    },
    UpperFunction: "upper", args => {
        if let Some(LiteralValue::Bytes(bytes)) = args.first() {
            let s = String::from_utf8_lossy(bytes).to_uppercase();
            Some(LiteralValue::Bytes(Arc::new(s.into_bytes())))
        } else {
            None
        }
    },
    LowerFunction: "lower", args => {
        if let Some(LiteralValue::Bytes(bytes)) = args.first() {
            let s = String::from_utf8_lossy(bytes).to_lowercase();
            Some(LiteralValue::Bytes(Arc::new(s.into_bytes())))
        } else {
            None
        }
    },
    SumFunction: "sum", args => {
        if let Some(LiteralValue::Array(arr)) = args.first() {
            let sum: i64 = arr.iter().filter_map(|v| if let LiteralValue::Int(i) = v { Some(*i) } else { None }).sum();
            Some(LiteralValue::Int(sum))
        } else {
            None
        }
    },
    StartsWithFunction: "starts_with", args => {
        if let (Some(LiteralValue::Bytes(haystack)), Some(LiteralValue::Bytes(prefix))) = (args.first(), args.get(1)) {
            let h = String::from_utf8_lossy(haystack);
            let p = String::from_utf8_lossy(prefix);
            Some(LiteralValue::Bool(h.starts_with(&*p)))
        } else {
            None
        }
    },
    EndsWithFunction: "ends_with", args => {
        if let (Some(LiteralValue::Bytes(haystack)), Some(LiteralValue::Bytes(suffix))) = (args.first(), args.get(1)) {
            let h = String::from_utf8_lossy(haystack);
            let s = String::from_utf8_lossy(suffix);
            Some(LiteralValue::Bool(h.ends_with(&*s)))
        } else {
            None
        }
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinFunctionId {
    Len,
    Upper,
    Lower,
    Sum,
    StartsWith,
    EndsWith,
}

impl BuiltinFunctionId {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "len" => Some(Self::Len),
            "upper" => Some(Self::Upper),
            "lower" => Some(Self::Lower),
            "sum" => Some(Self::Sum),
            "starts_with" => Some(Self::StartsWith),
            "ends_with" => Some(Self::EndsWith),
            _ => None,
        }
    }
}

pub fn call_builtin(id: BuiltinFunctionId, args: &[LiteralValue]) -> Option<LiteralValue> {
    match id {
        BuiltinFunctionId::Len => {
            if let Some(LiteralValue::Array(arr)) = args.first() {
                Some(LiteralValue::Int(arr.len() as i64))
            } else {
                None
            }
        }
        BuiltinFunctionId::Upper => {
            if let Some(LiteralValue::Bytes(bytes)) = args.first() {
                let s = String::from_utf8_lossy(bytes).to_uppercase();
                Some(LiteralValue::Bytes(Arc::new(s.into_bytes())))
            } else {
                None
            }
        }
        BuiltinFunctionId::Lower => {
            if let Some(LiteralValue::Bytes(bytes)) = args.first() {
                let s = String::from_utf8_lossy(bytes).to_lowercase();
                Some(LiteralValue::Bytes(Arc::new(s.into_bytes())))
            } else {
                None
            }
        }
        BuiltinFunctionId::Sum => {
            if let Some(LiteralValue::Array(arr)) = args.first() {
                let sum: i64 = arr.iter().filter_map(|v| if let LiteralValue::Int(i) = v { Some(*i) } else { None }).sum();
                Some(LiteralValue::Int(sum))
            } else {
                None
            }
        }
        BuiltinFunctionId::StartsWith => {
            if let (Some(LiteralValue::Bytes(haystack)), Some(LiteralValue::Bytes(prefix))) = (args.first(), args.get(1)) {
                let h = String::from_utf8_lossy(haystack);
                let p = String::from_utf8_lossy(prefix);
                Some(LiteralValue::Bool(h.starts_with(&*p)))
            } else {
                None
            }
        }
        BuiltinFunctionId::EndsWith => {
            if let (Some(LiteralValue::Bytes(haystack)), Some(LiteralValue::Bytes(suffix))) = (args.first(), args.get(1)) {
                let h = String::from_utf8_lossy(haystack);
                let s = String::from_utf8_lossy(suffix);
                Some(LiteralValue::Bool(h.ends_with(&*s)))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_register_and_call_len() {
        let mut reg = FunctionRegistry::new();
        reg.register("len", LenFunction);
        let arr = LiteralValue::Array(Arc::new(vec![LiteralValue::Int(1), LiteralValue::Int(2)]));
        let result = reg.get("len").unwrap().call(&[arr]);
        assert_eq!(result, Some(LiteralValue::Int(2)));
    }
    #[test]
    fn test_upper_function() {
        let mut reg = FunctionRegistry::new();
        reg.register("upper", UpperFunction);
        let val = LiteralValue::Bytes(Arc::new(b"hello".to_vec()));
        let result = reg.get("upper").unwrap().call(&[val]);
        assert_eq!(result, Some(LiteralValue::Bytes(Arc::new(b"HELLO".to_vec()))));
    }
    #[test]
    fn test_sum_function() {
        let mut reg = FunctionRegistry::new();
        reg.register("sum", SumFunction);
        let arr = LiteralValue::Array(Arc::new(vec![LiteralValue::Int(1), LiteralValue::Int(2), LiteralValue::Int(3)]));
        let result = reg.get("sum").unwrap().call(&[arr]);
        assert_eq!(result, Some(LiteralValue::Int(6)));
    }
    #[test]
    fn test_starts_with_function() {
        let mut reg = FunctionRegistry::new();
        reg.register("starts_with", StartsWithFunction);
        let val = LiteralValue::Bytes(Arc::new(b"foobar".to_vec()));
        let prefix = LiteralValue::Bytes(Arc::new(b"foo".to_vec()));
        let wrong = LiteralValue::Bytes(Arc::new(b"bar".to_vec()));
        assert_eq!(reg.get("starts_with").unwrap().call(&[val.clone(), prefix.clone()]), Some(LiteralValue::Bool(true)));
        assert_eq!(reg.get("starts_with").unwrap().call(&[val.clone(), wrong.clone()]), Some(LiteralValue::Bool(false)));
        assert_eq!(reg.get("starts_with").unwrap().call(&[wrong.clone(), prefix.clone()]), Some(LiteralValue::Bool(false)));
    }
    #[test]
    fn test_ends_with_function() {
        let mut reg = FunctionRegistry::new();
        reg.register("ends_with", EndsWithFunction);
        let val = LiteralValue::Bytes(Arc::new(b"foobar".to_vec()));
        let suffix = LiteralValue::Bytes(Arc::new(b"bar".to_vec()));
        let wrong = LiteralValue::Bytes(Arc::new(b"foo".to_vec()));
        assert_eq!(reg.get("ends_with").unwrap().call(&[val.clone(), suffix.clone()]), Some(LiteralValue::Bool(true)));
        assert_eq!(reg.get("ends_with").unwrap().call(&[val.clone(), wrong.clone()]), Some(LiteralValue::Bool(false)));
        assert_eq!(reg.get("ends_with").unwrap().call(&[wrong.clone(), suffix.clone()]), Some(LiteralValue::Bool(false)));
    }
    #[test]
    fn test_register_closure() {
        let mut reg = FunctionRegistry::new();
        reg.register_fn("always_true", |_args| Some(LiteralValue::Bool(true)));
        let result = reg.get("always_true").unwrap().call(&[]);
        assert_eq!(result, Some(LiteralValue::Bool(true)));
    }
} 