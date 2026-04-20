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
    Integer(i16),
    Bool(bool),
    Ident(String),
    // String(String), // TODO: do we need this?
    Array(Vec<Expr>),

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

    /// Array subscript (e.g. `arr[i]`)
    Subscript {
        array: Box<Expr>,
        index: Box<Expr>,
    },

    /// Member access (e.g. `obj.x`)
    MemberAccess {
        object: Box<Expr>,
        name: String,
    },

    /// Static access (e.g. `ivt::register`)
    StaticAccess {
        target: Box<Expr>,
        member: String,
    },

    /// Struct initialization (e.g. `Point { x, y }`)
    StructInit {
        // allows access like `namespace.User`
        name: Box<Expr>,
        fields: Vec<(String, Expr)>,
    },
}

pub type Expr = Spanned<ExprKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I16,
    Bool,
    Pointer(Box<Type>),
    Array { ty: Box<Type>, size: u16 },
    Struct(String),
    SelfType,
    Void,
    Error, // special error type used in the compiler
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

    Struct {
        name: String,
        members: Vec<Stmt>,
    },

    StructField {
        name: String,
        ty: Type,
    },
}

pub type Stmt = Spanned<StmtKind>;
