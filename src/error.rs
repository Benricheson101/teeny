use std::fmt;

use crate::parser::ast::Span;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error at {:?}: {}", self.span, self.message)
    }
}

pub type ParseResult<T> = Result<T, ParseError>;
