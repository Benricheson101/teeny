use crate::lexer::token::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn merge(a: Self, b: Self) -> Self {
        Self {
            start: a.start,
            end: b.end,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Integer(u16),

    Var(String),

    /// An expression with one operand (e.g. `-5`)
    Unary {
        op: TokenKind,
        rhs: Box<Expr>,
    },

    /// An expression with two operands (e.g. `6 + 7`)
    Binary {
        lhs: Box<Expr>,
        op: TokenKind,
        rhs: Box<Expr>,
    },

    /// assigning a value to a variable (e.g. `x = 5`, `y += 3`)
    Assignment {
        target: Box<Expr>,
        value: Box<Expr>,
    },
}

pub type Expr = Spanned<ExprKind>;
