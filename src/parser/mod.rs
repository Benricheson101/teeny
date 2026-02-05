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
            TokenKind::Integer(val) => {
                Spanned::new(ExprKind::Integer(*val), span)
            },

            TokenKind::Minus => {
                let right = self.parse_expr(Precedence::Prefix);
                let span = Span::merge(span, right.span);

                Spanned::new(
                    ExprKind::Unary {
                        op: TokenKind::Minus,
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
        let op_bp = Precedence::of(&op_token.kind);

        let rhs = self.parse_expr(op_bp);
        let span = Span::merge(lhs.span, rhs.span);

        Spanned::new(
            ExprKind::Binary {
                lhs: Box::new(lhs),
                op: op_token.kind,
                rhs: Box::new(rhs),
            },
            span,
        )
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
}
