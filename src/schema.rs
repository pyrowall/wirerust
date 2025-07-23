//! Schema module: defines the field/type registry for filters.
//!
//! This module provides the FilterSchema type and builder for defining available fields and types.

use crate::types::FieldType;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FilterSchema {
    fields: HashMap<String, FieldType>,
}

impl FilterSchema {
    pub fn get_field_type(&self, name: &str) -> Option<&FieldType> {
        self.fields.get(name)
    }
    pub fn fields(&self) -> &HashMap<String, FieldType> {
        &self.fields
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FilterSchemaBuilder {
    fields: HashMap<String, FieldType>,
}

impl FilterSchemaBuilder {
    pub fn new() -> Self {
        Self { fields: HashMap::new() }
    }
    pub fn field(mut self, name: impl Into<String>, ty: FieldType) -> Self {
        self.fields.insert(name.into(), ty);
        self
    }
    pub fn build(self) -> FilterSchema {
        FilterSchema { fields: self.fields }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FieldType;
    use serde_json;

    #[test]
    fn test_field_registration_and_retrieval() {
        let schema = FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .build();
        assert_eq!(schema.get_field_type("foo"), Some(&FieldType::Int));
        assert_eq!(schema.get_field_type("bar"), Some(&FieldType::Bytes));
        assert_eq!(schema.get_field_type("baz"), None);
    }

    #[test]
    fn test_fields_map() {
        let schema = FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .build();
        let fields = schema.fields();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains_key("foo"));
        assert!(fields.contains_key("bar"));
    }

    #[test]
    fn test_schema_serialization_deserialization() {
        let schema = FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Array(Box::new(FieldType::Bytes)))
            .build();
        let json = serde_json::to_string(&schema).unwrap();
        let deserialized: FilterSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(schema.fields(), deserialized.fields());
    }

    #[test]
    fn test_schema_builder_overwrite_field() {
        let schema = FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("foo", FieldType::Bytes)
            .build();
        // Last one wins
        assert_eq!(schema.get_field_type("foo"), Some(&FieldType::Bytes));
    }
} 