//! Types module: defines field and literal types for the filter engine.
//!
//! This module provides FieldType and LiteralValue enums, covering all supported types.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FieldType {
    Bytes,
    Int,
    Bool,
    Ip,
    Array(Box<FieldType>),
    Map(Box<FieldType>),
    Unknown, // Added for type inference failures
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LiteralValue {
    #[serde(serialize_with = "serialize_arc_vec_u8", deserialize_with = "deserialize_arc_vec_u8")]
    Bytes(Arc<Vec<u8>>),
    Int(i64),
    Bool(bool),
    Ip(IpAddr),
    #[serde(serialize_with = "serialize_arc_vec_lv", deserialize_with = "deserialize_arc_vec_lv")]
    Array(Arc<Vec<LiteralValue>>),
    #[serde(serialize_with = "serialize_arc_map_lv", deserialize_with = "deserialize_arc_map_lv")]
    Map(Arc<HashMap<String, LiteralValue>>),
}

impl PartialEq for LiteralValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LiteralValue::Bytes(a), LiteralValue::Bytes(b)) => a.as_slice() == b.as_slice(),
            (LiteralValue::Int(a), LiteralValue::Int(b)) => a == b,
            (LiteralValue::Bool(a), LiteralValue::Bool(b)) => a == b,
            (LiteralValue::Ip(a), LiteralValue::Ip(b)) => a == b,
            (LiteralValue::Array(a), LiteralValue::Array(b)) => a == b,
            (LiteralValue::Map(a), LiteralValue::Map(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for LiteralValue {}

fn serialize_arc_vec_u8<S>(arc: &Arc<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    serializer.serialize_bytes(arc)
}
fn deserialize_arc_vec_u8<'de, D>(deserializer: D) -> Result<Arc<Vec<u8>>, D::Error>
where D: Deserializer<'de> {
    let v: Vec<u8> = Deserialize::deserialize(deserializer)?;
    Ok(Arc::new(v))
}
fn serialize_arc_vec_lv<S>(arc: &Arc<Vec<LiteralValue>>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    let seq = arc.as_slice();
    seq.serialize(serializer)
}
fn deserialize_arc_vec_lv<'de, D>(deserializer: D) -> Result<Arc<Vec<LiteralValue>>, D::Error>
where D: Deserializer<'de> {
    let v: Vec<LiteralValue> = Deserialize::deserialize(deserializer)?;
    Ok(Arc::new(v))
}
fn serialize_arc_map_lv<S>(arc: &Arc<HashMap<String, LiteralValue>>, serializer: S) -> Result<S::Ok, S::Error>
where S: Serializer {
    let map = arc.as_ref();
    map.serialize(serializer)
}
fn deserialize_arc_map_lv<'de, D>(deserializer: D) -> Result<Arc<HashMap<String, LiteralValue>>, D::Error>
where D: Deserializer<'de> {
    let m: HashMap<String, LiteralValue> = Deserialize::deserialize(deserializer)?;
    Ok(Arc::new(m))
}

impl FieldType {
    pub fn is_primitive(&self) -> bool {
        matches!(self, FieldType::Bytes | FieldType::Int | FieldType::Bool | FieldType::Ip)
    }
}

impl LiteralValue {
    /// Infers the type of this literal value.
    /// For arrays/maps, if empty, returns Array(Unknown)/Map(Unknown) unless a hint is provided.
    pub fn get_type(&self) -> FieldType {
        self.get_type_with_hint(None)
    }

    /// Infers the type of this literal value, using a type hint for empty arrays/maps.
    /// If hint is Some(Array(T)), then empty arrays will be typed as Array(T) instead of Array(Unknown).
    pub fn get_type_with_hint(&self, hint: Option<&FieldType>) -> FieldType {
        match self {
            LiteralValue::Bytes(_) => FieldType::Bytes,
            LiteralValue::Int(_) => FieldType::Int,
            LiteralValue::Bool(_) => FieldType::Bool,
            LiteralValue::Ip(_) => FieldType::Ip,
            LiteralValue::Array(vals) => {
                let vals = &**vals;
                if vals.is_empty() {
                    if let Some(FieldType::Array(elem_ty)) = hint {
                        FieldType::Array(elem_ty.clone())
                    } else {
                        // If log crate is available, emit a warning here
                        // log::warn!("Type inference: empty array defaults to Unknown");
                        FieldType::Array(Box::new(FieldType::Unknown))
                    }
                } else {
                    let first_ty = vals[0].get_type();
                    if vals.iter().all(|v| v.get_type() == first_ty) {
                        FieldType::Array(Box::new(first_ty))
                    } else {
                        FieldType::Array(Box::new(FieldType::Unknown))
                    }
                }
            }
            LiteralValue::Map(map) => {
                let map = &**map;
                if map.is_empty() {
                    if let Some(FieldType::Map(val_ty)) = hint {
                        FieldType::Map(val_ty.clone())
                    } else {
                        // log::warn!("Type inference: empty map defaults to Unknown");
                        FieldType::Map(Box::new(FieldType::Unknown))
                    }
                } else {
                    let mut iter = map.values();
                    let first_ty = iter.next().map(|v| v.get_type());
                    if let Some(first_ty) = first_ty {
                        if iter.all(|v| v.get_type() == first_ty) {
                            FieldType::Map(Box::new(first_ty))
                        } else {
                            FieldType::Map(Box::new(FieldType::Unknown))
                        }
                    } else {
                        FieldType::Map(Box::new(FieldType::Unknown))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::str::FromStr;


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
        assert_eq!(LiteralValue::Bytes(Arc::new(b"abc".to_vec())).get_type(), FieldType::Bytes);
        assert_eq!(LiteralValue::Bool(true).get_type(), FieldType::Bool);
        let ip = IpAddr::from_str("127.0.0.1").unwrap();
        assert_eq!(LiteralValue::Ip(ip).get_type(), FieldType::Ip);
        let arr = LiteralValue::Array(Arc::new(vec![LiteralValue::Int(1), LiteralValue::Int(2)]));
        assert_eq!(arr.get_type(), FieldType::Array(Box::new(FieldType::Int)));
        let map = LiteralValue::Map(Arc::new(Default::default()));
        assert_eq!(map.get_type(), FieldType::Map(Box::new(FieldType::Unknown))); // Updated to match new logic
    }

    #[test]
    fn test_array_type_inference_empty() {
        let arr = LiteralValue::Array(Arc::new(vec![]));
        // Defaults to Bytes if empty
        assert_eq!(arr.get_type(), FieldType::Array(Box::new(FieldType::Unknown)));
    }

    #[test]
    fn test_map_type_inference_empty() {
        let map = LiteralValue::Map(Arc::new(Default::default()));
        assert_eq!(map.get_type(), FieldType::Map(Box::new(FieldType::Unknown)));
    }

    #[test]
    fn test_serialization_deserialization() {
        let ip = IpAddr::from_str("192.168.1.1").unwrap();
        let val = LiteralValue::Array(Arc::new(vec![
            LiteralValue::Int(1),
            LiteralValue::Bytes(Arc::new(b"foo".to_vec())),
            LiteralValue::Bool(false),
            LiteralValue::Ip(ip),
        ]));
        let json = serde_json::to_string(&val).unwrap();
        let deser: LiteralValue = serde_json::from_str(&json).unwrap();
        assert_eq!(val, deser);
    }
} 