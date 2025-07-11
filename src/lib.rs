//! Wirerust: A modular, embeddable filter engine for structured data.
//!
//! This crate is a clean, extensible reimplementation of the core ideas from Cloudflare's wirefilter.
//! It provides a way to define filter schemas, parse filter expressions, compile them, and execute them against runtime data.
//!
//! # Planned Architecture
//! - Schema definition (fields/types)
//! - Expression parsing (AST)
//! - Compilation to IR (closures or pluggable backends)
//! - Execution context (runtime values)
//! - Extensible function/type registry
//! - Optional FFI/WASM bindings

mod schema;
mod expr;
mod compiler;
mod filter;
mod context;
mod types;
mod functions;

pub use schema::*;
pub use expr::*;
pub use compiler::*;
pub use filter::*;
pub use context::*;
pub use types::*;
pub use functions::*;

use std::fmt;

/// Unified error type for Wirerust operations
#[derive(Debug)]
pub enum WirerustError {
    ParseError(String),
    TypeError(String),
    FunctionError(String),
    FieldNotFound(String),
    ExecutionError(String),
    Other(String),
}

impl fmt::Display for WirerustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WirerustError::ParseError(e) => write!(f, "Parse error: {}", e),
            WirerustError::TypeError(e) => write!(f, "Type error: {}", e),
            WirerustError::FunctionError(e) => write!(f, "Function error: {}", e),
            WirerustError::FieldNotFound(e) => write!(f, "Field not found: {}", e),
            WirerustError::ExecutionError(e) => write!(f, "Execution error: {}", e),
            WirerustError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

impl std::error::Error for WirerustError {}

// TODO: Update all modules to use Result<T, WirerustError> for public APIs.
