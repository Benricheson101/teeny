use crate::parser::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Precedence {
    /// default precedence level for everything except binary/unary ops
    Lowest,
    /// = += -=
    Assignment,
    /// ||
    LogicalOr,
    /// &&
    LogicalAnd,
    /// |
    BitOr,
    /// ^
    BitXor,
    /// &
    BitAnd,
    /// == !=
    Equality,
    /// < <= > >=
    Cmp,
    /// << >>
    BitShift,
    /// + -
    Sum,
    /// * /
    Product,
    /// -N
    Prefix,
    /// my_fn(x) [] . ++ --
    Call,
}

impl Precedence {
    /// importnat note: this is only for binary operators
    pub fn of(kind: &TokenKind) -> Self {
        use TokenKind::*;

        match kind {
            Equal | PlusEqual | MinusEqual | SlashEqual | StarEqual
            | PercentEqual => Self::Assignment,
            Or => Self::LogicalOr,
            And => Self::LogicalAnd,
            BitOr => Self::BitOr,
            BitXor => Self::BitXor,
            BitAnd => Self::BitAnd,
            Equality | BangEqual => Self::Equality,
            Gt | Gte | Lt | Lte => Self::Cmp,
            LeftShift | RightShift => Self::BitShift,
            Plus | Minus => Self::Sum,
            Star | Slash | Percent => Self::Product,
            LeftParen | PlusPlus | MinusMinus => Precedence::Call,
            _ => Self::Lowest,
        }
    }
}
