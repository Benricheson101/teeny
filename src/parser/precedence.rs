use crate::parser::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Precedence {
    /// default precedence level for everything except binary/unary ops
    Lowest,
    /// + -
    Sum,
    /// * /
    Product,
    /// -N
    Prefix,
    /// my_fn(x)
    Call,
}

impl Precedence {
    pub fn of(kind: &TokenKind) -> Self {
        match kind {
            TokenKind::Plus | TokenKind::Minus => Self::Sum,
            TokenKind::Star | TokenKind::Slash => Self::Product,
            TokenKind::LeftParen => Precedence::Call,
            _ => Self::Lowest,
        }
    }
}
