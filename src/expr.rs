//! Expression (AST) module: defines the parsed representation of filter expressions.
//!
//! This module provides the FilterExpr type and related AST node types.

use crate::types::LiteralValue;
use crate::schema::FilterSchema;
use serde::{Serialize, Deserialize};
use crate::WirerustError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FilterExpr {
    LogicalOp {
        op: LogicalOp,
        left: Box<FilterExpr>,
        right: Box<FilterExpr>,
    },
    Comparison {
        left: Box<FilterExpr>,
        op: ComparisonOp,
        right: Box<FilterExpr>,
    },
    Not(Box<FilterExpr>),
    Value(LiteralValue),
    FunctionCall {
        name: String,
        args: Vec<FilterExpr>,
    },
    List(Vec<LiteralValue>),
    // TODO: Add more as needed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ComparisonOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    In,
    NotIn,
    Matches, // for regex
    Wildcard, // case-insensitive wildcard
    StrictWildcard, // case-sensitive wildcard
    Contains, // substring or element containment
}

// Visitor trait for traversing the AST
pub trait ExprVisitor {
    fn visit(&mut self, expr: &FilterExpr);
}

// Hand-written recursive descent parser for filter expressions
pub struct FilterParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> FilterParser<'a> {
    pub fn new(input: &'a str, _schema: &'a FilterSchema) -> Self {
        Self { input, pos: 0 }
    }

    pub fn parse(input: &str, schema: &FilterSchema) -> Result<FilterExpr, WirerustError> {
        let mut parser = FilterParser::new(input, schema);
        let expr = parser.parse_expr().map_err(|e| WirerustError::ParseError(format!("Failed to parse expression at position {}: {e}", parser.pos)))?;
        parser.skip_whitespace();
        if parser.pos < parser.input.len() {
            return Err(WirerustError::ParseError(format!("Unexpected input at position {}", parser.pos)));
        }
        Ok(expr)
    }

    fn parse_expr(&mut self) -> Result<FilterExpr, WirerustError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<FilterExpr, WirerustError> {
        self.skip_whitespace();
        let mut left = self.parse_and()?;
        loop {
            self.skip_whitespace();
            if self.consume("||") || self.consume("or") {
                self.skip_whitespace();
                let right = { self.skip_whitespace(); self.parse_and()? };
                left = FilterExpr::LogicalOp {
                    op: LogicalOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<FilterExpr, WirerustError> {
        self.skip_whitespace();
        let mut left = self.parse_not()?;
        loop {
            self.skip_whitespace();
            if self.consume("&&") || self.consume("and") {
                self.skip_whitespace();
                let right = { self.skip_whitespace(); self.parse_not()? };
                left = FilterExpr::LogicalOp {
                    op: LogicalOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<FilterExpr, WirerustError> {
        self.skip_whitespace();
        if self.consume("not") {
            let expr = self.parse_not()?;
            Ok(FilterExpr::Not(Box::new(expr)))
        } else {
            self.parse_comparison()
        }
    }

    fn parse_expr_or_value(&mut self) -> Result<FilterExpr, WirerustError> {
        self.skip_whitespace();
        // Try to parse as a literal first, then as an identifier, then as a full expression
        let start_pos = self.pos;
        
        // Try literal first (most specific)
        if let Ok(lit) = self.parse_literal() {
            return Ok(FilterExpr::Value(lit));
        }
        self.pos = start_pos;
        
        // Try identifier (field reference)
        if let Ok(ident) = self.parse_identifier() {
            return Ok(FilterExpr::Value(LiteralValue::Bytes(ident.into_bytes())));
        }
        self.pos = start_pos;
        
        // Try full expression last (least specific)
        if let Ok(expr) = self.parse_expr() {
            return Ok(expr);
        }
        
        Err(WirerustError::ParseError(format!("Expected expression or value at position {}", self.pos)))
    }

    fn parse_comparison(&mut self) -> Result<FilterExpr, WirerustError> {
        self.skip_whitespace();
        // Parse primary expression: identifier, function call, or parenthesized expression
        let left = if self.peek() == Some('(') {
            self.consume_char();
            let inner = self.parse_expr()?;
            self.skip_whitespace();
            if !self.consume(")") {
                return Err(WirerustError::ParseError(format!("Expected ')' at position {}", self.pos)));
            }
            inner
        } else {
            // Parse identifier or function call
            let ident = self.parse_identifier()?;
            self.skip_whitespace();
            if self.peek() == Some('(') {
                // Function call
                self.consume_char();
                let mut args = Vec::new();
                self.skip_whitespace();
                if self.peek() != Some(')') {
                    loop {
                        // Try to parse as a simple field reference first, then as a full expression
                        let start_pos = self.pos;
                        let arg = if let Ok(ident) = self.parse_identifier() {
                            // Simple field reference
                            FilterExpr::Value(LiteralValue::Bytes(ident.into_bytes()))
                        } else {
                            // Reset and try as full expression
                            self.pos = start_pos;
                            self.parse_expr_or_value()?
                        };
                        args.push(arg);
                        self.skip_whitespace();
                        if self.peek() == Some(',') {
                            self.consume_char();
                            self.skip_whitespace();
                        } else {
                            break;
                        }
                    }
                }
                if !self.consume(")") {
                    return Err(WirerustError::ParseError(format!("Expected ')' after function call at position {}", self.pos)));
                }
                FilterExpr::FunctionCall { name: ident, args }
            } else if ident == "{" {
                let list = self.parse_list_literal()?;
                FilterExpr::List(list)
            } else {
                // Just an identifier (field reference)
                FilterExpr::Value(LiteralValue::Bytes(ident.into_bytes()))
            }
        };
        self.skip_whitespace();
        // Check for comparison operator
        if let Ok((op, _op_str)) = self.parse_operator() {
            self.skip_whitespace();
            let right = if self.peek() == Some('{') {
                // List/set literal as value
                let list = self.parse_list_literal()?;
                self.skip_whitespace();
                FilterExpr::Value(LiteralValue::Array(list))
            } else {
                // Try to parse as a full expression or value
                self.parse_expr_or_value()?
            };
            Ok(FilterExpr::Comparison {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_identifier(&mut self) -> Result<String, WirerustError> {
        self.skip_whitespace();
        let start = self.pos;
        let mut end = self.pos;
        for (i, c) in self.input[self.pos..].char_indices() {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                end = self.pos + i + c.len_utf8();
            } else {
                break;
            }
        }
        if end > start {
            let ident = &self.input[start..end];
            self.pos = end;
            Ok(ident.to_string())
        } else {
            Err(WirerustError::ParseError(format!("Expected identifier at position {}", self.pos)))
        }
    }

    fn parse_operator(&mut self) -> Result<(ComparisonOp, &'static str), WirerustError> {
        let ops = [
            ("==", ComparisonOp::Eq),
            ("eq", ComparisonOp::Eq),
            ("!=", ComparisonOp::Neq),
            ("ne", ComparisonOp::Neq),
            ("<=", ComparisonOp::Lte),
            ("le", ComparisonOp::Lte),
            (">=", ComparisonOp::Gte),
            ("ge", ComparisonOp::Gte),
            ("<", ComparisonOp::Lt),
            ("lt", ComparisonOp::Lt),
            (">", ComparisonOp::Gt),
            ("gt", ComparisonOp::Gt),
            ("in", ComparisonOp::In),
            ("not in", ComparisonOp::NotIn),
            ("matches", ComparisonOp::Matches),
            ("wildcard", ComparisonOp::Wildcard),
            ("strict wildcard", ComparisonOp::StrictWildcard),
            ("contains", ComparisonOp::Contains),
        ];
        self.skip_whitespace();
        for (s, op) in ops.iter() {
            if self.input[self.pos..].starts_with(s) {
                self.pos += s.len();
                return Ok((*op, *s));
            }
        }
        Err(WirerustError::ParseError(format!("Expected operator at position {}", self.pos)))
    }

    fn parse_literal(&mut self) -> Result<LiteralValue, WirerustError> {
        self.skip_whitespace();
        if let Some(c) = self.peek() {
            if c == '"' {
                return self.parse_string_literal();
            } else if c.is_ascii_digit() || c == '-' {
                return self.parse_int_literal();
            } else if self.input[self.pos..].starts_with("true") {
                self.pos += 4;
                return Ok(LiteralValue::Bool(true));
            } else if self.input[self.pos..].starts_with("false") {
                self.pos += 5;
                return Ok(LiteralValue::Bool(false));
            }
        }
        Err(WirerustError::ParseError(format!("Expected literal at position {}", self.pos)))
    }

    fn parse_string_literal(&mut self) -> Result<LiteralValue, WirerustError> {
        self.skip_whitespace();
        if self.peek() != Some('"') {
            return Err(WirerustError::ParseError(format!("Expected \" at position {}", self.pos)));
        }
        self.consume_char(); // consume opening quote
        let start = self.pos;
        let mut end = self.pos;
        while let Some(c) = self.peek() {
            if c == '"' {
                break;
            }
            self.consume_char();
            end = self.pos;
        }
        if self.peek() != Some('"') {
            return Err(WirerustError::ParseError(format!("Unterminated string literal at position {}", self.pos)));
        }
        let s = &self.input[start..end];
        self.consume_char(); // consume closing quote
        Ok(LiteralValue::Bytes(s.as_bytes().to_vec()))
    }

    fn parse_int_literal(&mut self) -> Result<LiteralValue, WirerustError> {
        self.skip_whitespace();
        let start = self.pos;
        if self.peek() == Some('-') {
            self.consume_char();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.consume_char();
            } else {
                break;
            }
        }
        if self.pos > start {
            let s = &self.input[start..self.pos];
            match s.parse::<i64>() {
                Ok(n) => Ok(LiteralValue::Int(n)),
                Err(_) => Err(WirerustError::ParseError(format!("Invalid integer literal at position {}", start))),
            }
        } else {
            Err(WirerustError::ParseError(format!("Expected integer literal at position {}", self.pos)))
        }
    }

    fn parse_list_literal(&mut self) -> Result<Vec<LiteralValue>, WirerustError> {
        if !self.consume("{") {
            return Err(WirerustError::ParseError(format!("Expected '{{' at position {}", self.pos)));
        }
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some('}') {
                self.consume_char();
                break;
            }
            let item = self.parse_literal()?;
            items.push(item);
            self.skip_whitespace();
            // Accept either whitespace or comma as separator, but do not require comma
            // If next is '}', break, else continue
        }
        Ok(items)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.consume_char();
            } else {
                break;
            }
        }
    }

    fn consume(&mut self, s: &str) -> bool {
        if self.input[self.pos..].as_bytes().starts_with(s.as_bytes()) {
            self.pos += s.len();
            true
        } else {
            false
        }
    }

    fn consume_char(&mut self) -> Option<char> {
        let mut iter = self.input[self.pos..].char_indices();
        let (offset, ch) = iter.next()?;
        self.pos += offset + ch.len_utf8();
        Some(ch)
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FieldType;
    use crate::schema::FilterSchemaBuilder;

    fn schema() -> FilterSchema {
        FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .build()
    }

    #[test]
    fn test_parse_comparison() {
        let expr = FilterParser::parse("foo == 42", &schema()).unwrap();
        match expr {
            FilterExpr::Comparison { left, op, right } => {
                assert_eq!(*left, FilterExpr::Value(LiteralValue::Bytes(b"foo".to_vec())));
                assert_eq!(op, ComparisonOp::Eq);
                assert_eq!(*right, FilterExpr::Value(LiteralValue::Int(42)));
            }
            _ => panic!("Expected comparison expr"),
        }
    }

    #[test]
    fn test_parse_comparison_word_operators() {
        let sch = schema();
        let eq = FilterParser::parse("foo eq 42", &sch).unwrap();
        match eq {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Eq),
            _ => panic!("Expected eq comparison"),
        }
        let ne = FilterParser::parse("foo ne 42", &sch).unwrap();
        match ne {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Neq),
            _ => panic!("Expected ne comparison"),
        }
        let lt = FilterParser::parse("foo lt 42", &sch).unwrap();
        match lt {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Lt),
            _ => panic!("Expected lt comparison"),
        }
        let le = FilterParser::parse("foo le 42", &sch).unwrap();
        match le {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Lte),
            _ => panic!("Expected le comparison"),
        }
        let gt = FilterParser::parse("foo gt 42", &sch).unwrap();
        match gt {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Gt),
            _ => panic!("Expected gt comparison"),
        }
        let ge = FilterParser::parse("foo ge 42", &sch).unwrap();
        match ge {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Gte),
            _ => panic!("Expected ge comparison"),
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let expr = FilterParser::parse("foo == 1 && bar == \"baz\"", &schema()).unwrap();
        match expr {
            FilterExpr::LogicalOp { op, .. } => assert_eq!(op, LogicalOp::And),
            _ => panic!("Expected logical op"),
        }
        let expr_word = FilterParser::parse("foo == 1 and bar == \"baz\"", &schema()).unwrap();
        match expr_word {
            FilterExpr::LogicalOp { op, .. } => assert_eq!(op, LogicalOp::And),
            _ => panic!("Expected logical op for 'and'"),
        }
    }

    #[test]
    fn test_parse_logical_or() {
        let expr = FilterParser::parse("foo == 1 || bar == \"baz\"", &schema()).unwrap();
        match expr {
            FilterExpr::LogicalOp { op, .. } => assert_eq!(op, LogicalOp::Or),
            _ => panic!("Expected logical op"),
        }
        let expr_word = FilterParser::parse("foo == 1 or bar == \"baz\"", &schema()).unwrap();
        match expr_word {
            FilterExpr::LogicalOp { op, .. } => assert_eq!(op, LogicalOp::Or),
            _ => panic!("Expected logical op for 'or'"),
        }
    }

    #[test]
    fn test_parse_not() {
        let expr = FilterParser::parse("not foo == 0", &schema()).unwrap();
        match expr {
            FilterExpr::Not(inner) => match *inner {
                FilterExpr::Comparison { .. } => {}
                _ => panic!("Expected comparison inside not"),
            },
            _ => panic!("Expected not expr"),
        }
    }

    #[test]
    fn test_parse_parens() {
        let expr = FilterParser::parse("(foo == 1 || bar == \"baz\") && foo != 0", &schema()).unwrap();
        match expr {
            FilterExpr::LogicalOp { op: LogicalOp::And, .. } => {},
            _ => panic!("Expected top-level and"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let expr = FilterParser::parse("myfunc(foo, 42)", &schema()).unwrap();
        match expr {
            FilterExpr::FunctionCall { name, args } => {
                assert_eq!(name, "myfunc");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected function call expr"),
        }
    }

    #[test]
    fn test_parse_list_literal() {
        let expr = FilterParser::parse("foo in {1 2 3}", &schema()).unwrap();
        match expr {
            FilterExpr::Comparison { op, right, .. } => {
                assert_eq!(op, ComparisonOp::In);
                match *right {
                    FilterExpr::Value(LiteralValue::Array(ref arr)) => {
                        assert_eq!(arr.len(), 3);
                    }
                    _ => panic!("Expected array literal"),
                }
            }
            _ => panic!("Expected comparison expr with list literal"),
        }
    }

    #[test]
    fn test_parse_wildcard_operators() {
        let sch = schema();
        let wc = FilterParser::parse("bar wildcard \"foo*bar\"", &sch).unwrap();
        match wc {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Wildcard),
            _ => panic!("Expected wildcard comparison"),
        }
        let swc = FilterParser::parse("bar strict wildcard \"foo*bar\"", &sch).unwrap();
        match swc {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::StrictWildcard),
            _ => panic!("Expected strict wildcard comparison"),
        }
    }

    #[test]
    fn test_parse_contains_operator() {
        let sch = schema();
        let expr = FilterParser::parse("bar contains \"foo\"", &sch).unwrap();
        match expr {
            FilterExpr::Comparison { op, .. } => assert_eq!(op, ComparisonOp::Contains),
            _ => panic!("Expected contains comparison"),
        }
    }
} 