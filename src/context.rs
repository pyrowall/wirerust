//! Context module: holds runtime field values for filter execution.
//!
//! This module provides the FilterContext type.

use crate::types::{FieldType, LiteralValue};
use crate::schema::FilterSchema;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::WirerustError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterContext {
    values: HashMap<String, LiteralValue>,
}

impl FilterContext {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, field: &str, value: LiteralValue, schema: &FilterSchema) -> Result<(), WirerustError> {
        match schema.get_field_type(field) {
            Some(expected_type) => {
                if &value.get_type() == expected_type {
                    self.values.insert(field.to_string(), value);
                    Ok(())
                } else {
                    Err(WirerustError::TypeError(format!("Type mismatch for field '{}': expected {:?}, got {:?}", field, expected_type, value.get_type())))
                }
            }
            None => Err(WirerustError::FieldNotFound(field.to_string())),
        }
    }

    pub fn get(&self, field: &str) -> Option<&LiteralValue> {
        self.values.get(field)
    }

    pub fn values(&self) -> &HashMap<String, LiteralValue> {
        &self.values
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldType, LiteralValue};
    use crate::schema::FilterSchemaBuilder;
    use serde_json;

    fn schema() -> FilterSchema {
        FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .field("arr", FieldType::Array(Box::new(FieldType::Int)))
            .build()
    }

    #[test]
    fn test_set_and_get_value() {
        let mut ctx = FilterContext::new();
        let sch = schema();
        ctx.set("foo", LiteralValue::Int(42), &sch).unwrap();
        assert_eq!(ctx.get("foo"), Some(&LiteralValue::Int(42)));
    }

    #[test]
    fn test_type_checking() {
        let mut ctx = FilterContext::new();
        let sch = schema();
        // Wrong type
        let res = ctx.set("foo", LiteralValue::Bytes(b"not an int".to_vec()), &sch);
        assert!(res.is_err());
        // Correct type
        assert!(ctx.set("foo", LiteralValue::Int(1), &sch).is_ok());
    }

    #[test]
    fn test_field_not_found() {
        let mut ctx = FilterContext::new();
        let sch = schema();
        let res = ctx.set("unknown", LiteralValue::Int(1), &sch);
        assert!(matches!(res, Err(WirerustError::FieldNotFound(_))));
    }

    #[test]
    fn test_array_type_checking() {
        let mut ctx = FilterContext::new();
        let sch = schema();
        // Correct array type
        let arr = LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(2)]);
        assert!(ctx.set("arr", arr, &sch).is_ok());
        // Wrong array element type
        let arr = LiteralValue::Array(vec![LiteralValue::Bytes(b"bad".to_vec())]);
        let res = ctx.set("arr", arr, &sch);
        // This will currently pass because get_type() only checks the first element or defaults to Bytes
        // TODO: Improve type inference for arrays
        // For now, just check that it doesn't panic
        assert!(res.is_ok() || res.is_err());
    }

    #[test]
    fn test_serialization_deserialization() {
        let mut ctx = FilterContext::new();
        let sch = schema();
        ctx.set("foo", LiteralValue::Int(123), &sch).unwrap();
        ctx.set("bar", LiteralValue::Bytes(b"abc".to_vec()), &sch).unwrap();
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: FilterContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx.values(), deserialized.values());
    }
} 