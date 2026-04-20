#![allow(unused_assignments)] // miette Diagnostic breaks this somehow

use std::{error::Error, fmt};

use miette::{Diagnostic, NamedSource, Report};

use crate::{
    lexer::token::Token,
    parser::ast::{Span, Type},
};

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

#[derive(Debug, Clone)]
pub enum TeenyCompilerErrorKind {
    ExpectedIdentifier(Token),
    ExpectedInteger(Token),
    ExpectedType(Token),
    ExpectedToken(Token, Token),
    UnexpectedToken(Token),
    SyntaxError,

    IdentNotDefined(String),
    InvalidFnScope,
    InvalidStructScope,

    TypeMismatch(Type, Type),
    CannotInferType,
    ParamCountMismatch(usize, usize),
}

impl fmt::Display for TeenyCompilerErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TeenyCompilerErrorKind::ExpectedIdentifier(token) => {
                write!(f, "Expected identifier, got {token:?}")
            },
            TeenyCompilerErrorKind::ExpectedInteger(token) => {
                write!(f, "Expected integer, got {token:?}")
            },
            TeenyCompilerErrorKind::ExpectedType(token) => {
                write!(f, "Expected type, got {token:?}")
            },
            TeenyCompilerErrorKind::ExpectedToken(expected, got) => {
                write!(f, "Expected token {expected:?}, got {got:?}")
            },
            TeenyCompilerErrorKind::UnexpectedToken(token) => {
                write!(f, "Unexpected token {token:?}")
            },
            TeenyCompilerErrorKind::SyntaxError => write!(f, "Syntax error"),
            TeenyCompilerErrorKind::IdentNotDefined(name) => {
                write!(f, "{name} is not defined")
            },
            TeenyCompilerErrorKind::InvalidFnScope => {
                write!(f, "Functions can only be declared in the global scope")
            },
            TeenyCompilerErrorKind::InvalidStructScope => {
                write!(f, "Structs can only be declared in the global scope")
            },
            TeenyCompilerErrorKind::TypeMismatch(a, b) => {
                use miette::NamedSource;
                write!(f, "Type mismatch, expected {a:?} got {b:?}")
            },
            TeenyCompilerErrorKind::CannotInferType => {
                write!(f, "Unable to infer type")
            },
            TeenyCompilerErrorKind::ParamCountMismatch(a, b) => write!(
                f,
                "Incorrect number of parameters. Expected {a}, got {b}"
            ),
        }
    }
}

#[derive(Debug, Clone, Diagnostic)]
pub struct TeenyCompilerError {
    #[label("here")]
    pub span: Span,
    pub kind: TeenyCompilerErrorKind,
}

impl fmt::Display for TeenyCompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error: {}", self.kind)
    }
}

impl Error for TeenyCompilerError {
}

pub type TeenyResult<T> = Result<T, TeenyCompilerError>;

pub fn print_errors<T>(source: &str, filename: &str, errors: &[T])
where
    T: Diagnostic + Send + Sync + Clone + 'static,
{
    for err in errors {
        let source = NamedSource::new(filename, source.to_string());
        let err = Report::new(err.clone()).with_source_code(source);
        eprintln!("{:?}", err);
    }
}
