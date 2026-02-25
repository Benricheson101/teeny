#![allow(unused_assignments)] // miette Diagnostic breaks this somehow

use std::{error::Error, fmt};

use miette::Diagnostic;

use crate::parser::ast::Span;

#[derive(Debug, Clone, Diagnostic)]
pub struct ParseError {
    pub message: String,
    #[label("here")]
    pub span: Span,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.message)
    }
}

impl Error for ParseError {
}

pub type ParseResult<T> = Result<T, ParseError>;
