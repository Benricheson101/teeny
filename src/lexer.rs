use std::{iter::Peekable, str::Chars};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    LeftParen,
    RightParen,

    Minus,
    Plus,
    Star,
    Slash,

    Integer(u16),

    Illegal(char),

    EOF
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
    /// Extracts the raw text for the token from the input string
    pub fn lexeme<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start .. self.end]
    }
}

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    source: Peekable<Chars<'a>>,

    // used for error messages
    line: usize,
    col: usize,

    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.chars().peekable(),
            line: 1,
            col: 1,
            pos: 0,
        }
    }

    /// Consumes one character of input and updates the tracked positions
    fn advance(&mut self) -> Option<char> {
        let ch = self.source.next();

        if let Some(c) = ch {
            self.pos += c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }

        ch
    }

    /// Eats all whitespace
    fn eat_whitespace(&mut self) {
        while let Some(&ch) = self.source.peek() && ch.is_whitespace() {
            self.advance();
        }
    }

    /// Reads a full non-negative integer from the stream
    fn read_integer(&mut self) -> u16 {
        let mut number_str = String::new();
        while let Some(&ch) = self.source.peek() && ch.is_ascii_digit() {
            number_str.push(ch);
            self.advance();
        }

        number_str.parse().unwrap()
    }

    /// Tokenizes the next complete token
    fn next_token(&mut self) -> Token {
        use TokenKind::*;

        self.eat_whitespace();

        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.col;


        let kind = match self.source.peek() {
            Some(&ch) => match ch {
                '+' => {
                    self.advance();
                    Plus
                },

                '-' => {
                    self.advance();
                    Minus
                },

                '*' => {
                    self.advance();
                    Star
                },

                '/' => {
                    self.advance();
                    Slash
                },

                '(' => {
                    self.advance();
                    LeftParen
                },

                ')' => {
                    self.advance();
                    RightParen
                },

                _ if ch.is_ascii_digit() => Integer(self.read_integer()),

                _ => {
                    let invalid = self.advance().unwrap();
                    Illegal(invalid)
                }
            },

            None => EOF,
        };

        Token {
            kind,
            line: start_line,
            col: start_col,
            start: start_pos,
            end: self.pos,
        }

    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token() {
            Token {kind: TokenKind::EOF, ..} => None,
            t @ _ => Some(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_basic_math() {
        let input = "10 + 5";
        let mut lex = Lexer::new(input);

        let t1 = lex.next().unwrap();
        assert_eq!(t1.kind, TokenKind::Integer(10));
        assert_eq!(t1.line, 1);
        assert_eq!(t1.col, 1);
        assert_eq!(t1.start, 0);
        assert_eq!(t1.end, 2);

        let t2 = lex.next().unwrap();
        assert_eq!(t2.kind, TokenKind::Plus);

        let t3 = lex.next().unwrap();
        assert_eq!(t3.kind, TokenKind::Integer(5));

        let t4 = lex.next();
        assert_eq!(t4, None);
    }

    #[test]
    fn tokenize_newlines() {
        let input = " \n   +";
        let mut lex = Lexer::new(input);

        let t1 = lex.next().unwrap();
        assert_eq!(t1.kind, TokenKind::Plus);
        assert_eq!(t1.line, 2);
        assert_eq!(t1.col, 4);
    }

    #[test]
    fn tokenize_lexemes() {
        let input = "100 + 200";
        let mut lex = Lexer::new(input);

        let t1 = lex.next().unwrap();
        assert_eq!(t1.lexeme(input), "100");

        lex.next().unwrap();

        let t2 = lex.next().unwrap();
        assert_eq!(t2.lexeme(input), "200");
    }
}
