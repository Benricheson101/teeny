pub mod token;
pub mod keywords;

use std::{iter::Peekable, str::Chars};

use token::{Token, TokenKind};

use crate::lexer::keywords::KEYWORDS;

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    source: Peekable<Chars<'a>>,
    source_string: &'a str,

    // used for error messages
    line: usize,
    col: usize,

    start: usize,
    pos: usize,
    eof: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source_string: source,
            source: source.chars().peekable(),
            line: 1,
            col: 1,
            start: 0,
            pos: 0,
            eof: false,
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

    fn is_at_end(&mut self) -> bool {
        self.source.peek().is_none()
    }

    fn next_matches(&mut self, c: char) -> bool {
        if let Some(ch) = self.source.peek()
            && *ch == c
        {
            self.advance();
            true
        } else {
            false
        }
    }

    fn eat_whitespace(&mut self) {
        while let Some(&ch) = self.source.peek()
            && ch.is_whitespace()
        {
            self.advance();
        }
    }

    fn integer(&mut self) -> u16 {
        while let Some(ch) = self.source.peek()
            && ch.is_ascii_digit()
        {
            self.advance();
        }

        self.source_string[self.start..self.pos].parse().unwrap()
    }

    fn ident(&mut self) -> TokenKind {
        while self.source.peek().is_some_and(|p| p.is_ascii_alphanumeric()) {
            self.advance();
        }

        let txt = &self.source_string[self.start .. self.pos];
        if let Some(kw) = KEYWORDS.get(txt) {
            kw.to_owned()
        } else {
            TokenKind::Ident(txt.to_owned())
        }
    }

    fn read_token(&mut self) -> Token {
        use crate::lexer::token::TokenKind::*;

        loop {
            self.eat_whitespace();

            self.start = self.pos;

            let start_line = self.line;
            let start_col = self.col;

            let ch = self.advance();

            let kind = match ch {
                Some(ch) => match ch {
                    '(' => LeftParen,
                    ')' => RightParen,
                    '{' => LeftBrace,
                    '}' => RightBrace,
                    '[' => LeftBracket,
                    ']' => RightBracket,
                    ';' => Semi,
                    '~' => BitNot,
                    '^' => BitXor,

                    '&' if self.next_matches('&') => And,
                    '&' => BitAnd,

                    '|' if self.next_matches('|') => Or,
                    '|' => BitOr,

                    '!' if self.next_matches('=') => BangEqual,
                    '!' => Bang,

                    '+' if self.next_matches('=') => PlusEqual,
                    '+' if self.next_matches('+') => PlusPlus,
                    '+' => Plus,

                    '-' if self.next_matches('=') => MinusEqual,
                    '-' if self.next_matches('-') => MinusMinus,
                    '-' => Minus,

                    '*' if self.next_matches('=') => StarEqual,
                    '*' => Star,

                    '/' if self.next_matches('/') => {
                        while self.source.peek().is_some_and(|p| *p != '\n')
                            && !self.is_at_end()
                        {
                            self.advance();
                        }
                        continue;
                    },
                    '/' if self.next_matches('=') => SlashEqual,
                    '/' => Slash,

                    '<' if self.next_matches('=') => Lte,
                    '<' if self.next_matches('<') => LeftShift,
                    '<' => Lt,

                    '>' if self.next_matches('=') => Gte,
                    '>' if self.next_matches('>') => RightShift,
                    '>' => Gt,

                    _ if ch.is_ascii_digit() => Integer(self.integer()),

                    _ if ch.is_ascii_alphabetic() => {
                        self.ident()
                    },

                    _ => panic!("illegal character: {ch}"),
                },

                None => Eof,
            };

            return Token {
                kind,
                line: start_line,
                col: start_col,
                start: self.start,
                end: self.pos,
            };
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        let token = self.read_token();

        // adds an EOF token instead of ending the iterator
        if let TokenKind::Eof = token.kind {
            self.eof = true;
        }

        Some(token)
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

        let t4 = lex.next().unwrap();
        assert_eq!(t4.kind, TokenKind::Eof);

        let t5 = lex.next();
        assert_eq!(t5, None);
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

    #[test]
    fn tokenize_iterator() {
        let input = "100 + 200";
        let lex = Lexer::new(input);
        let tokens: Vec<_> = lex.collect();

        let expected = vec![
            Token {
                kind: TokenKind::Integer(100),
                line: 1,
                col: 1,
                start: 0,
                end: 3,
            },
            Token {
                kind: TokenKind::Plus,
                line: 1,
                col: 5,
                start: 4,
                end: 5,
            },
            Token {
                kind: TokenKind::Integer(200),
                line: 1,
                col: 7,
                start: 6,
                end: 9,
            },
            Token {
                kind: TokenKind::Eof,
                line: 1,
                col: 10,
                start: 9,
                end: 9,
            },
        ];

        assert_eq!(tokens.len(), expected.len());

        for i in 0..tokens.len() {
            assert_eq!(tokens[i], expected[i]);
        }
    }

    #[test]
    fn skip_comments() {
        let lex = Lexer::new("2 // 5 comment blah");
        let tokens: Vec<_> = lex.collect();

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, TokenKind::Integer(2));
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn tokenize_multi_char_operators() {
        let lex = Lexer::new("+= ++ + -- *= >> <= <");
        let tokens: Vec<_> = lex.collect();

        assert_eq!(tokens[0].start, 0);
        assert_eq!(tokens[0].end, 2);

        assert_eq!(tokens[0].kind, TokenKind::PlusEqual);
        assert_eq!(tokens[1].kind, TokenKind::PlusPlus);
        assert_eq!(tokens[2].kind, TokenKind::Plus);
        assert_eq!(tokens[3].kind, TokenKind::MinusMinus);
        assert_eq!(tokens[4].kind, TokenKind::StarEqual);
        assert_eq!(tokens[5].kind, TokenKind::RightShift);
        assert_eq!(tokens[6].kind, TokenKind::Lte);
        assert_eq!(tokens[7].kind, TokenKind::Lt);
        assert_eq!(tokens[8].kind, TokenKind::Eof);
    }

    #[test]
    fn tokenize_identifiers() {
        let lex = Lexer::new("hello hi5");
        let tokens: Vec<_> = lex.collect();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, TokenKind::Ident("hello".to_string()));
        assert_eq!(tokens[0].start, 0);
        assert_eq!(tokens[0].end, 5);

        assert_eq!(tokens[1].kind, TokenKind::Ident("hi5".to_string()));
        assert_eq!(tokens[1].start, 6);
        assert_eq!(tokens[1].end, 9);
    }

    #[test]
    fn tokenize_keywords() {
        let lex = Lexer::new("if else return for while");
        let tokens: Vec<_> = lex.collect();

        assert_eq!(tokens[0].kind, TokenKind::If);
        assert_eq!(tokens[1].kind, TokenKind::Else);
        assert_eq!(tokens[2].kind, TokenKind::Return);
        assert_eq!(tokens[3].kind, TokenKind::For);
        assert_eq!(tokens[4].kind, TokenKind::While);
        assert_eq!(tokens[5].kind, TokenKind::Eof);
    }
}
