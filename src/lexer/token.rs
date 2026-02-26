#[derive(Debug, Clone, PartialEq, Eq)]
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
    Percent,

    BitAnd,
    BitXor,
    BitOr,
    BitNot,
    RightShift,
    LeftShift,

    Equality,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    BangEqual,
    PercentEqual,

    And,
    Or,

    PlusPlus,
    MinusMinus,

    Gt,
    Gte,
    Lt,
    Lte,

    Equal,

    Semi,
    Comma,
    Dot,
    Colon,
    ColonColon,
    Arrow,

    Integer(i16),

    // keywords
    Fn,
    Let,
    Const,
    Return,
    If,
    Else,
    While,
    For,
    Repeat,
    Struct,
    True,
    False,

    Ident(String),

    // types
    Bool,
    I16,

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
