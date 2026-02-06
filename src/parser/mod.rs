use ast::{Expr, ExprKind, Span, Spanned};

use crate::{
    lexer::token::{Token, TokenKind},
    parser::precedence::Precedence,
};

pub mod ast;
pub mod precedence;

#[derive(Debug, Clone)]
pub struct Parser {
    source: Vec<Token>,
    cur: usize,
}

impl Parser {
    pub fn new(source: Vec<Token>) -> Self {
        Self { source, cur: 0 }
    }

    fn peek(&self) -> &Token {
        &self.source[self.cur]
    }

    fn advance(&mut self) -> &Token {
        let token = &self.source[self.cur];
        if token.kind != TokenKind::Eof {
            self.cur += 1;
        }

        token
    }

    fn consume(&mut self, expected: TokenKind) -> Token {
        let token = self.peek();

        if token.kind != expected {
            panic!(
                "Expected token {:?} but got {:?} at line {}, col {}",
                expected, token.kind, token.line, token.col
            );
        }

        self.advance().clone()
    }

    fn consume_ident(&mut self) -> String {
        let token = self.peek();

        match &token.kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                name
            },
            _ => panic!(
                "Expected identifier, got {:?} at line {}",
                token.kind, token.line
            ),
        }
    }

    fn parse_expr_list(&mut self, term: TokenKind) -> Vec<Expr> {
        let mut args = Vec::new();

        if self.peek().kind == term {
            self.advance();
            return args;
        }

        loop {
            args.push(self.parse_expr(Precedence::Lowest));
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            } else {
                break;
            }
        }

        self.consume(term);

        args
    }

    // -- pratt parsing --

    pub fn parse_expr(&mut self, bp: Precedence) -> Expr {
        let mut left = self.parse_prefix();

        while bp < Precedence::of(&self.peek().kind) {
            left = self.parse_infix(left);
        }

        left
    }

    fn parse_prefix(&mut self) -> Expr {
        let token = self.advance();
        let span = Span::new(token.start, token.end);

        match &token.kind {
            TokenKind::True => Expr::new(ExprKind::Bool(true), span),
            TokenKind::False => Expr::new(ExprKind::Bool(false), span),

            TokenKind::Integer(val) => {
                Spanned::new(ExprKind::Integer(*val), span)
            },

            TokenKind::Ident(name) => {
                Spanned::new(ExprKind::Ident(name.clone()), span)
            },

            TokenKind::LeftBracket => {
                let elems = self.parse_expr_list(TokenKind::RightBracket);
                Expr::new(ExprKind::Array(elems), span)
            },

            tk @ (TokenKind::Minus | TokenKind::Bang) => {
                let tk = tk.clone();
                let right = self.parse_expr(Precedence::Prefix);
                let span = Span::merge(span, right.span);

                Spanned::new(
                    ExprKind::Unary {
                        op: tk,
                        rhs: Box::new(right),
                    },
                    span,
                )
            },

            TokenKind::LeftParen => {
                let expr = self.parse_expr(Precedence::Lowest);

                if let TokenKind::RightParen = self.peek().kind {
                    self.advance();
                } else {
                    panic!("Expected ')' at {}", self.peek().start);
                }

                expr
            },

            _ => panic!("Unexpected token: {token:?}"),
        }
    }

    fn parse_infix(&mut self, lhs: Expr) -> Expr {
        let op_token = self.advance().clone();

        match op_token.kind {
            TokenKind::Equal
            | TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual
            | TokenKind::SlashEqual => {
                let value = self.parse_expr(Precedence::Lowest);
                if !matches!(lhs.node, ExprKind::Ident(_)) {
                    panic!("Assigning to a non-variable: {lhs:?}");
                }

                let span = Span::merge(lhs.span, value.span);
                Expr::new(
                    ExprKind::Assignment {
                        target: Box::new(lhs),
                        value: Box::new(value),
                    },
                    span,
                )
            },

            TokenKind::LeftParen => {
                let args = self.parse_expr_list(TokenKind::RightParen);
                let span =
                    Span::new(lhs.span.start, self.source[self.cur - 1].end);

                Expr::new(
                    ExprKind::Call {
                        callee: Box::new(lhs),
                        args,
                    },
                    span,
                )
            },

            TokenKind::LeftBracket => {
                let index = self.parse_expr(Precedence::Lowest);
                self.consume(TokenKind::RightBracket);
                let span = Span::merge(lhs.span, index.span);

                Expr::new(
                    ExprKind::Subscript {
                        array: Box::new(lhs),
                        index: Box::new(index),
                    },
                    span,
                )
            },

            TokenKind::Dot => {
                let name = self.consume_ident();
                let span =
                    Span::new(lhs.span.start, self.source[self.cur - 1].end);

                Expr::new(
                    ExprKind::MemberAccess {
                        object: Box::new(lhs),
                        name,
                    },
                    span,
                )
            },

            _ => {
                let op_bp = Precedence::of(&op_token.kind);
                let rhs = self.parse_expr(op_bp);
                let span = Span::merge(lhs.span, rhs.span);

                Expr::new(
                    ExprKind::Binary {
                        lhs: Box::new(lhs),
                        op: op_token.kind,
                        rhs: Box::new(rhs),
                    },
                    span,
                )
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_expr(input: &str) -> Expr {
        let lex = Lexer::new(input);
        let tokens: Vec<_> = lex.collect();
        let mut parser = Parser::new(tokens);
        parser.parse_expr(Precedence::Lowest)
    }

    #[test]
    fn integer_literal() {
        let expr = parse_expr("123");
        assert_eq!(expr.node, ExprKind::Integer(123));
    }

    #[test]
    fn prefix_operator() {
        let expr = parse_expr("-5");

        let expected = ExprKind::Unary {
            op: TokenKind::Minus,
            rhs: Box::new(Expr::new(ExprKind::Integer(5), Span::new(1, 2))),
        };

        assert_eq!(expr.node, expected);
    }

    #[test]
    fn infix_operator() {
        let expr = parse_expr("5 + 10");

        let expected = ExprKind::Binary {
            lhs: Box::new(Expr::new(ExprKind::Integer(5), Span::new(0, 1))),

            op: TokenKind::Plus,

            rhs: Box::new(Expr::new(ExprKind::Integer(10), Span::new(4, 6))),
        };

        assert_eq!(expr.node, expected);
    }

    #[test]
    fn operator_precedence() {
        let expr = parse_expr("1 + 2 * 3");

        match expr.node {
            ExprKind::Binary { lhs, op, rhs } => {
                assert_eq!(op, TokenKind::Plus);
                assert_eq!(lhs.node, ExprKind::Integer(1));

                match rhs.node {
                    ExprKind::Binary { lhs, op, rhs } => {
                        assert_eq!(op, TokenKind::Star);
                        assert_eq!(lhs.node, ExprKind::Integer(2));
                        assert_eq!(rhs.node, ExprKind::Integer(3));
                    },
                    _ => panic!("Right side is not a binary expr"),
                }
            },
            _ => panic!("Top level is not a binary expr"),
        }
    }

    #[test]
    fn grouped_expr() {
        let expr = parse_expr("(1 + 2) * 3");

        match expr.node {
            ExprKind::Binary { lhs, op, rhs } => {
                assert_eq!(op, TokenKind::Star);
                assert_eq!(rhs.node, ExprKind::Integer(3));

                match lhs.node {
                    ExprKind::Binary { lhs, op, rhs } => {
                        assert_eq!(op, TokenKind::Plus);
                        assert_eq!(lhs.node, ExprKind::Integer(1));
                        assert_eq!(rhs.node, ExprKind::Integer(2));
                    },
                    _ => panic!("Right side is not a binary expr"),
                }
            },
            _ => panic!("Top level is not a binary expr"),
        }
    }

    #[test]
    fn variable_in_expr() {
        let expr = parse_expr("x + 5");
        let expected = ExprKind::Binary {
            lhs: Box::new(Expr::new(
                ExprKind::Ident("x".to_string()),
                Span::new(0, 1),
            )),
            op: TokenKind::Plus,
            rhs: Box::new(Expr::new(ExprKind::Integer(5), Span::new(4, 5))),
        };

        assert_eq!(expr.node, expected);
    }

    #[test]
    fn assignment() {
        let expr = parse_expr("var = 1");

        match expr.node {
            ExprKind::Assignment { target, value } => {
                assert_eq!(target.node, ExprKind::Ident("var".to_string()));
                assert_eq!(value.node, ExprKind::Integer(1));
            },
            _ => panic!("top level is not Assignment"),
        }
    }

    #[test]
    fn parse_call() {
        let expr = parse_expr("something(3)");
        let expected = ExprKind::Call {
            callee: Box::new(Expr::new(
                ExprKind::Ident("something".to_string()),
                Span::new(0, 9),
            )),
            args: vec![Expr::new(ExprKind::Integer(3), Span::new(10, 11))],
        };

        assert_eq!(expr.node, expected);
    }

    #[test]
    fn parse_subscript() {
        let expr = parse_expr("arr[5]");
        let expected = ExprKind::Subscript {
            array: Box::new(Expr::new(
                ExprKind::Ident("arr".to_string()),
                Span::new(0, 3),
            )),
            index: Box::new(Expr::new(ExprKind::Integer(5), Span::new(4, 5))),
        };

        assert_eq!(expr.node, expected);
    }

    #[test]
    fn parse_member_accessor() {
        let expr = parse_expr("user.name");
        let expected = ExprKind::MemberAccess {
            object: Box::new(Expr::new(
                ExprKind::Ident("user".to_string()),
                Span::new(0, 4),
            )),
            name: "name".to_string(),
        };

        assert_eq!(expr.node, expected);
    }

    #[test]
    fn parse_booleans() {
        let expr = parse_expr("true");
        let expected = ExprKind::Bool(true);
        assert_eq!(expr.node, expected);
    }

    #[test]
    fn parse_array_literal() {
        let expr = parse_expr("[1, 2]");
        let expected = ExprKind::Array(vec![
            Expr::new(ExprKind::Integer(1), Span::new(1, 2)),
            Expr::new(ExprKind::Integer(2), Span::new(4, 5)),
        ]);
        assert_eq!(expr.node, expected);
    }
}
