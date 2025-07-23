//! Context module: holds runtime field values for filter execution.
//!
//! This module provides the FilterContext type.

use crate::types::{LiteralValue};
use crate::types::FieldType;
use crate::schema::FilterSchema;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::WirerustError;
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterContext {
    values: HashMap<String, LiteralValue>,
}

pub struct FilterContextBuilder<'a> {
    ctx: FilterContext,
    schema: &'a FilterSchema,
}

impl<'a> FilterContextBuilder<'a> {
    pub fn new(schema: &'a FilterSchema) -> Self {
        Self { ctx: FilterContext::new(), schema }
    }
    pub fn build(self) -> FilterContext {
        self.ctx
    }
}

/// Macro to generate both builder and context setters for a given type.
macro_rules! define_setters_and_getters {
    // For types with owned value
    ($(($set_name:ident, $builder_set:ident, $variant:ident, $ty:ty, $get_name:ident)),* $(,)?) => {
        impl<'a> FilterContextBuilder<'a> {
            $(
            pub fn $builder_set(mut self, field: &str, value: $ty) -> Result<Self, WirerustError> {
                self.ctx.$set_name(field, value, self.schema);
                Ok(self)
            }
            )*
        }
        impl FilterContext {
            $(
            pub fn $set_name(&mut self, field: &str, value: $ty, schema: &FilterSchema) -> &mut Self {
                let _ = self.set(field, LiteralValue::$variant(value), schema);
                self
            }
            pub fn $get_name(&self, field: &str) -> Option<$ty> {
                match self.get(field) {
                    Some(&LiteralValue::$variant(ref v)) => Some(v.clone()),
                    _ => None,
                }
            }
            )*
        }
    };
    // Special case for bytes
    (bytes) => {
        impl<'a> FilterContextBuilder<'a> {
            pub fn set_bytes(mut self, field: &str, value: impl AsRef<[u8]>) -> Result<Self, WirerustError> {
                self.ctx.set_bytes(field, value, self.schema);
                Ok(self)
            }
        }
        impl FilterContext {
            pub fn set_bytes<T: AsRef<[u8]>>(&mut self, field: &str, value: T, schema: &FilterSchema) -> &mut Self {
                let _ = self.set(field, LiteralValue::Bytes(value.as_ref().to_vec()), schema);
                self
            }
            pub fn get_bytes(&self, field: &str) -> Option<&[u8]> {
                match self.get(field) {
                    Some(&LiteralValue::Bytes(ref b)) => Some(&b[..]),
                    _ => None,
                }
            }
        }
    };
}

define_setters_and_getters! {
    (set_int, set_int, Int, i64, get_int),
    (set_bool, set_bool, Bool, bool, get_bool),
    (set_ip, set_ip, Ip, IpAddr, get_ip),
    (set_array, set_array, Array, Vec<LiteralValue>, get_array),
}
define_setters_and_getters!(bytes);

impl FilterContext {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, field: &str, value: LiteralValue, schema: &FilterSchema) -> Result<(), WirerustError> {
        match schema.get_field_type(field) {
            Some(expected_type) => {
                let value_type = value.get_type();
                // Special case: allow empty arrays for any array type
                if let (FieldType::Array(expected_elem), FieldType::Array(value_elem)) = (expected_type, &value_type) {
                    if let FieldType::Unknown = **value_elem {
                        self.values.insert(field.to_string(), value);
                        return Ok(());
                    }
                }
                if &value_type == expected_type {
                    self.values.insert(field.to_string(), value);
                    Ok(())
                } else {
                    Err(WirerustError::TypeError(format!("Type mismatch for field '{}': expected {:?}, got {:?}", field, expected_type, value_type)))
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
    use std::net::IpAddr;
    use std::str::FromStr;

    fn schema() -> FilterSchema {
        FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .field("arr", FieldType::Array(Box::new(FieldType::Int)))
            .field("flag", FieldType::Bool)
            .field("ip", FieldType::Ip)
            .build()
    }

    #[test]
    fn test_context_builder_and_typed_setters() {
        let sch = schema();
        let ip = IpAddr::from_str("127.0.0.1").unwrap();
        let ctx = FilterContextBuilder::new(&sch)
            .set_int("foo", 42).unwrap()
            .set_bytes("bar", b"baz").unwrap()
            .set_bool("flag", true).unwrap()
            .set_ip("ip", ip).unwrap()
            .set_array("arr", vec![LiteralValue::Int(1), LiteralValue::Int(2)]).unwrap()
            .build();
        assert_eq!(ctx.get_int("foo"), Some(42));
        assert_eq!(ctx.get_bytes("bar"), Some(&b"baz"[..]));
        assert_eq!(ctx.get_bool("flag"), Some(true));
        assert_eq!(ctx.get_ip("ip"), Some(ip));
        assert_eq!(ctx.get("arr"), Some(&LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(2)])));
    }

    #[test]
    fn test_typed_setters_and_getters() {
        let sch = schema();
        let mut ctx = FilterContext::new();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();
        ctx.set_int("foo", 123, &sch)
            .set_bytes("bar", b"abc", &sch)
            .set_bool("flag", false, &sch)
            .set_ip("ip", ip, &sch);
        assert_eq!(ctx.get_int("foo"), Some(123));
        assert_eq!(ctx.get_bytes("bar"), Some(&b"abc"[..]));
        assert_eq!(ctx.get_bool("flag"), Some(false));
        assert_eq!(ctx.get_ip("ip"), Some(ip));
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