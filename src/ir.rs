//! Intermediate Representation (IR) for the filter engine.
//!
//! This module defines the bytecode instructions and supporting types for fast filter execution.

use crate::types::LiteralValue;

/// Unique identifier for a field in the schema.
pub type FieldId = usize;
/// Unique identifier for a function in the registry.
pub type FunctionId = usize;

/// A single instruction in the filter bytecode.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Push the value of a field onto the stack.
    LoadField(FieldId),
    /// Push a literal value onto the stack.
    LoadLiteral(LiteralValue),
    /// Call a function with N arguments (popped from the stack).
    CallFunction(FunctionId, u8),
    /// Comparison operations (pop two, push result).
    CompareEq,
    CompareNeq,
    CompareLt,
    CompareLte,
    CompareGt,
    CompareGte,
    CompareIn,
    CompareNotIn,
    CompareMatches,
    CompareWildcard { strict: bool },
    CompareContains,
    /// Logical operations.
    LogicalAnd,
    LogicalOr,
    LogicalNot,
}

/// The IR stack used during interpretation.
pub type IrStack = Vec<LiteralValue>; 