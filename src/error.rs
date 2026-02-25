#![allow(unused_assignments)] // miette Diagnostic breaks this somehow

use std::{error::Error, fmt};

use miette::Diagnostic;

use crate::{lexer::token::Token, parser::ast::Span};

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
            TeenyCompilerErrorKind::SyntaxError => todo!(),
            TeenyCompilerErrorKind::IdentNotDefined(name) => {
                write!(f, "{name} is not defined")
            },
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
