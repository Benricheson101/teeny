#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,

    Minus,
    Plus,
    Star,
    Slash,
    Bang,

    BitAnd,
    BitXor,
    BitOr,
    BitNot,
    RightShift,
    LeftShift,

    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    BangEqual,

    And,
    Or,

    PlusPlus,
    MinusMinus,

    Gt,
    Gte,
    Lt,
    Lte,

    Semi,

    Integer(u16),

    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
    pub start: usize,
    pub end: usize,
}

impl Token {
    /// Extract the raw text for the token from the input string
    pub fn lexeme<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}
