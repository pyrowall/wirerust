//! Types module: defines field and literal types for the filter engine.
//!
//! This module provides FieldType and LiteralValue enums, covering all supported types.

use std::collections::HashMap;
use std::net::IpAddr;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldType {
    Bytes,
    Int,
    Bool,
    Ip,
    Array(Box<FieldType>),
    Map(Box<FieldType>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
    Bytes(Vec<u8>),
    Int(i64),
    Bool(bool),
    Ip(IpAddr),
    Array(Vec<LiteralValue>),
    Map(HashMap<String, LiteralValue>),
}

impl FieldType {
    pub fn is_primitive(&self) -> bool {
        matches!(self, FieldType::Bytes | FieldType::Int | FieldType::Bool | FieldType::Ip)
    }
}

impl LiteralValue {
    pub fn get_type(&self) -> FieldType {
        match self {
            LiteralValue::Bytes(_) => FieldType::Bytes,
            LiteralValue::Int(_) => FieldType::Int,
            LiteralValue::Bool(_) => FieldType::Bool,
            LiteralValue::Ip(_) => FieldType::Ip,
            LiteralValue::Array(vals) => {
                let ty = vals.get(0).map(|v| v.get_type()).unwrap_or(FieldType::Bytes);
                FieldType::Array(Box::new(ty))
            }
            LiteralValue::Map(_) => FieldType::Map(Box::new(FieldType::Bytes)), // TODO: infer value type
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;
    use serde_json;

    #[test]
    fn test_field_type_is_primitive() {
        assert!(FieldType::Int.is_primitive());
        assert!(FieldType::Bytes.is_primitive());
        assert!(FieldType::Bool.is_primitive());
        assert!(FieldType::Ip.is_primitive());
        assert!(!FieldType::Array(Box::new(FieldType::Int)).is_primitive());
        assert!(!FieldType::Map(Box::new(FieldType::Int)).is_primitive());
    }

    #[test]
    fn test_literal_value_get_type() {
        assert_eq!(LiteralValue::Int(1).get_type(), FieldType::Int);
        assert_eq!(LiteralValue::Bytes(b"abc".to_vec()).get_type(), FieldType::Bytes);
        assert_eq!(LiteralValue::Bool(true).get_type(), FieldType::Bool);
        let ip = IpAddr::from_str("127.0.0.1").unwrap();
        assert_eq!(LiteralValue::Ip(ip).get_type(), FieldType::Ip);
        let arr = LiteralValue::Array(vec![LiteralValue::Int(1), LiteralValue::Int(2)]);
        assert_eq!(arr.get_type(), FieldType::Array(Box::new(FieldType::Int)));
        let map = LiteralValue::Map(Default::default());
        assert_eq!(map.get_type(), FieldType::Map(Box::new(FieldType::Bytes))); // TODO: improve value type inference
    }

    #[test]
    fn test_array_type_inference_empty() {
        let arr = LiteralValue::Array(vec![]);
        // Defaults to Bytes if empty
        assert_eq!(arr.get_type(), FieldType::Array(Box::new(FieldType::Bytes)));
    }

    #[test]
    fn test_map_type_inference_empty() {
        let map = LiteralValue::Map(Default::default());
        assert_eq!(map.get_type(), FieldType::Map(Box::new(FieldType::Bytes)));
    }

    #[test]
    fn test_serialization_deserialization() {
        let ip = IpAddr::from_str("192.168.1.1").unwrap();
        let val = LiteralValue::Array(vec![
            LiteralValue::Int(1),
            LiteralValue::Bytes(b"foo".to_vec()),
            LiteralValue::Bool(false),
            LiteralValue::Ip(ip),
        ]);
        let json = serde_json::to_string(&val).unwrap();
        let deser: LiteralValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deser);
    }
} 