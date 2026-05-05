use miette::SourceSpan;

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

impl From<Span> for SourceSpan {
    fn from(s: Span) -> Self {
        Self::new(s.start.into(), s.end - s.start)
    }
}

use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

impl NodeId {
    pub fn next() -> Self {
        NodeId(NEXT_NODE_ID.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
    pub id: NodeId,
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.span == other.span
    }
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self {
            node,
            span,
            id: NodeId::next(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Integer(u16),
    Bool(bool),
    Ident(String),

    /// `x++`, `++x`
    Increment {
        /// true when `++x`
        prefix: bool,
        expr: Box<Expr>,
    },

    /// `x--`, `--x`
    Decrement {
        /// true when `--x`
        prefix: bool,
        expr: Box<Expr>,
    },

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

    /// Assigning a value to a variable (e.g. `x = 5`, `y += 3`)
    Assignment {
        target: Box<Expr>,
        value: Box<Expr>,
    },

    /// A function call (e.g. `my_fn(1, 2)`)
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
}

pub type Expr = Spanned<ExprKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Bool,
    Pointer(Box<Type>),
    Void,
    Error, // special error type used in the compiler
}

impl Type {
    pub fn size(&self) -> u16 {
        match self {
            Type::Int => 1,
            Type::Bool => 1,
            Type::Pointer(_) => 1,
            Type::Void => 0,
            Type::Error => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    VarDecl {
        name: String,
        ty: Option<Type>,
        value: Expr,
        mutable: bool,
    },

    Expr(Expr),

    Block(Vec<Stmt>),

    Return(Option<Expr>),

    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },

    While {
        cond: Expr,
        body: Box<Stmt>,
    },

    Fn {
        name: String,
        params: Vec<(String, Type)>,
        return_type: Option<Type>,
        body: Box<Stmt>,
    },
}

pub type Stmt = Spanned<StmtKind>;
