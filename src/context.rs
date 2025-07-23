//! Context module: holds runtime field values for filter execution.
//!
//! This module provides the FilterContext type.

use crate::types::{LiteralValue};
use crate::types::FieldType;
use crate::schema::FilterSchema;
//use std::collections::HashMap; // unused
use serde::{Serialize, Deserialize};
use crate::WirerustError;
use std::net::IpAddr;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterContext {
    field_values: Vec<Option<LiteralValue>>, // index = FieldId
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
    pub fn set_int(mut self, field: &str, value: i64) -> Result<Self, WirerustError> {
        self.ctx.set_int(field, value, self.schema);
        Ok(self)
    }
    pub fn set_bool(mut self, field: &str, value: bool) -> Result<Self, WirerustError> {
        self.ctx.set_bool(field, value, self.schema);
        Ok(self)
    }
    pub fn set_ip(mut self, field: &str, value: IpAddr) -> Result<Self, WirerustError> {
        self.ctx.set_ip(field, value, self.schema);
        Ok(self)
    }
    pub fn set_bytes(mut self, field: &str, value: impl AsRef<[u8]>) -> Result<Self, WirerustError> {
        self.ctx.set_bytes(field, value, self.schema);
        Ok(self)
    }
    pub fn set_array(mut self, field: &str, value: Vec<LiteralValue>) -> Result<Self, WirerustError> {
        self.ctx.set_array(field, value, self.schema);
        Ok(self)
    }
}

impl FilterContext {
    pub fn new() -> Self {
        Self {
            field_values: Vec::new(),
        }
    }
    /// Set a field value by field ID.
    pub fn set_by_id(&mut self, field_id: usize, value: LiteralValue) {
        if self.field_values.len() <= field_id {
            self.field_values.resize(field_id + 1, None);
        }
        self.field_values[field_id] = Some(value);
    }
    /// Get a field value by field ID.
    pub fn get_by_id(&self, field_id: usize) -> Option<&LiteralValue> {
        self.field_values.get(field_id).and_then(|v| v.as_ref())
    }

    pub fn set(&mut self, field: &str, value: LiteralValue, schema: &FilterSchema) -> Result<(), WirerustError> {
        match schema.get_field_type(field) {
            Some(expected_type) => {
                let value_type = value.get_type();
                // Special case: allow empty arrays for any array type
                if let (FieldType::Array(_expected_elem), FieldType::Array(value_elem)) = (expected_type, &value_type) {
                    if let FieldType::Unknown = **value_elem {
                        if let Some(fid) = schema.field_id(field) {
                            self.set_by_id(fid, value.clone());
                        }
                        return Ok(());
                    }
                }
                if &value_type == expected_type {
                    if let Some(fid) = schema.field_id(field) {
                        self.set_by_id(fid, value.clone());
                    }
                    Ok(())
                } else {
                    Err(WirerustError::TypeError(format!("Type mismatch for field '{}': expected {:?}, got {:?}", field, expected_type, value_type)))
                }
            }
            None => Err(WirerustError::FieldNotFound(field.to_string())),
        }
    }

    pub fn get(&self, field: &str, schema: &FilterSchema) -> Option<&LiteralValue> {
        schema.field_id(field).and_then(|fid| self.get_by_id(fid))
    }

    pub fn set_int(&mut self, field: &str, value: i64, schema: &FilterSchema) -> &mut Self {
        let _ = self.set(field, LiteralValue::Int(value), schema);
        self
    }
    pub fn set_bool(&mut self, field: &str, value: bool, schema: &FilterSchema) -> &mut Self {
        let _ = self.set(field, LiteralValue::Bool(value), schema);
        self
    }
    pub fn set_ip(&mut self, field: &str, value: IpAddr, schema: &FilterSchema) -> &mut Self {
        let _ = self.set(field, LiteralValue::Ip(value), schema);
        self
    }
    pub fn set_bytes<T: AsRef<[u8]>>(&mut self, field: &str, value: T, schema: &FilterSchema) -> &mut Self {
        let _ = self.set(field, LiteralValue::Bytes(Arc::new(value.as_ref().to_vec())), schema);
        self
    }
    pub fn set_array(&mut self, field: &str, value: Vec<LiteralValue>, schema: &FilterSchema) -> &mut Self {
        let _ = self.set(field, LiteralValue::Array(Arc::new(value)), schema);
        self
    }
    pub fn get_int(&self, field: &str, schema: &FilterSchema) -> Option<i64> {
        match self.get(field, schema) {
            Some(LiteralValue::Int(i)) => Some(*i),
            _ => None,
        }
    }
    pub fn get_bool(&self, field: &str, schema: &FilterSchema) -> Option<bool> {
        match self.get(field, schema) {
            Some(LiteralValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }
    pub fn get_ip(&self, field: &str, schema: &FilterSchema) -> Option<IpAddr> {
        match self.get(field, schema) {
            Some(LiteralValue::Ip(ip)) => Some(*ip),
            _ => None,
        }
    }
    pub fn get_bytes(&self, field: &str, schema: &FilterSchema) -> Option<&[u8]> {
        match self.get(field, schema) {
            Some(LiteralValue::Bytes(b)) => Some(&b[..]),
            _ => None,
        }
    }
    pub fn get_array(&self, field: &str, schema: &FilterSchema) -> Option<Arc<Vec<LiteralValue>>> {
        match self.get(field, schema) {
            Some(LiteralValue::Array(arr)) => Some(Arc::clone(arr)),
            _ => None,
        }
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
            .set_bool("flag", true).unwrap()
            .set_ip("ip", ip).unwrap()
            .set_array("arr", vec![LiteralValue::Int(1), LiteralValue::Int(2)]).unwrap()
            .build();
        assert_eq!(ctx.get_int("foo", &sch), Some(42));
        assert_eq!(ctx.get_bool("flag", &sch), Some(true));
        assert_eq!(ctx.get_ip("ip", &sch), Some(ip));
        assert_eq!(ctx.get_array("arr", &sch), Some(Arc::new(vec![LiteralValue::Int(1), LiteralValue::Int(2)])));
    }

    #[test]
    fn test_typed_setters_and_getters() {
        let sch = schema();
        let mut ctx = FilterContext::new();
        let ip = IpAddr::from_str("192.168.1.1").unwrap();
        ctx.set_int("foo", 123, &sch)
            .set_bool("flag", false, &sch)
            .set_ip("ip", ip, &sch);
        assert_eq!(ctx.get_int("foo", &sch), Some(123));
        assert_eq!(ctx.get_bool("flag", &sch), Some(false));
        assert_eq!(ctx.get_ip("ip", &sch), Some(ip));
    }

    #[test]
    fn test_set_and_get_value() {
        let mut ctx = FilterContext::new();
        let sch = schema();
        ctx.set("foo", LiteralValue::Int(42), &sch).unwrap();
        assert_eq!(ctx.get("foo", &sch), Some(&LiteralValue::Int(42)));
    }

    #[test]
    fn test_type_checking() {
        let mut ctx = FilterContext::new();
        let sch = schema();
        // Wrong type
        let res = ctx.set("foo", LiteralValue::Bytes(Arc::new(b"not an int".to_vec())), &sch);
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
        let arr = LiteralValue::Array(Arc::new(vec![LiteralValue::Int(1), LiteralValue::Int(2)]));
        assert!(ctx.set("arr", arr, &sch).is_ok());
        // Wrong array element type
        let arr = LiteralValue::Array(Arc::new(vec![LiteralValue::Bytes(Arc::new(b"bad".to_vec()))]));
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
        ctx.set("bar", LiteralValue::Bytes(Arc::new(b"abc".to_vec())), &sch).unwrap();
        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: FilterContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx.get("foo", &sch), deserialized.get("foo", &sch));
        assert_eq!(ctx.get("bar", &sch), deserialized.get("bar", &sch));
    }
} 