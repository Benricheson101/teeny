use ast::{Expr, ExprKind, Span, Spanned};

use crate::{
    lexer::token::{Token, TokenKind},
    parser::{
        ast::{Stmt, StmtKind, Type},
        precedence::Precedence,
    },
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

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();

        while !self.is_at_end() {
            stmts.push(self.parse_stmt());
        }

        stmts
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

    fn matches(&mut self, ty: TokenKind) -> bool {
        if self.check(&ty) {
            self.advance();
            return true;
        }
        false
    }

    fn matches_any(&mut self, types: &[TokenKind]) -> bool {
        for ty in types {
            if self.check(ty) {
                self.advance();
                return true;
            }
        }

        false
    }

    fn check(&self, ty: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }

        &self.peek().kind == ty
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
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

    fn consume_integer(&mut self) -> i16 {
        let token = self.peek();

        match token.kind {
            TokenKind::Integer(val) => {
                self.advance();
                val
            },
            _ => panic!(
                "Expected integer, got {:?} at line {}",
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

    // -- recursive descent parser for statements --

    pub(crate) fn parse_stmt(&mut self) -> Stmt {
        let token = self.peek();

        match token.kind {
            TokenKind::Let | TokenKind::Const => self.parse_var_decl(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::LeftBrace => self.parse_block(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Fn => self.parse_fn_decl(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_expr_stmt(&mut self) -> Stmt {
        let expr = self.parse_expr(Precedence::Lowest);
        let semi = self.consume(TokenKind::Semi);
        let span = Span::new(expr.span.start, semi.end);

        Stmt::new(StmtKind::Expr(expr), span)
    }

    fn parse_type(&mut self) -> Type {
        if self.matches(TokenKind::Star) {
            return Type::Pointer(Box::new(self.parse_type()));
        }

        if self.matches(TokenKind::LeftBracket) {
            let elem_type = self.parse_type();
            self.consume(TokenKind::Semi);

            let size = self.consume_integer();

            self.consume(TokenKind::RightBracket);

            return Type::Array {
                ty: Box::new(elem_type),
                size: size as u16,
            };
        }

        match &self.advance().kind {
            TokenKind::I16 => Type::I16,
            TokenKind::Bool => Type::Bool,
            TokenKind::Ident(name) => Type::Struct(name.clone()),
            t @ _ => panic!("Expected type, got {:?}", t),
        }
    }

    fn parse_block(&mut self) -> Stmt {
        let start = self.consume(TokenKind::LeftBrace);

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            stmts.push(self.parse_stmt());
        }

        let end = self.consume(TokenKind::RightBrace);

        let span = Span::new(start.start, end.end);

        Stmt::new(StmtKind::Block(stmts), span)
    }

    fn parse_if(&mut self) -> Stmt {
        let start = self.consume(TokenKind::If);

        self.consume(TokenKind::LeftParen);
        let cond = self.parse_expr(Precedence::Lowest);
        self.consume(TokenKind::RightParen);

        let then_branch = Box::new(self.parse_stmt());

        let else_branch = if self.matches(TokenKind::Else) {
            Some(Box::new(self.parse_stmt()))
        } else {
            None
        };

        let end_pos = if let Some(ref else_branch) = else_branch {
            else_branch.span.end
        } else {
            then_branch.span.end
        };

        let span = Span::new(start.start, end_pos);

        Stmt::new(
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            },
            span,
        )
    }

    fn parse_while(&mut self) -> Stmt {
        let start = self.consume(TokenKind::While);

        self.consume(TokenKind::LeftParen);
        let cond = self.parse_expr(Precedence::Lowest);
        self.consume(TokenKind::RightParen);

        let body = Box::new(self.parse_stmt());

        let span = Span::new(start.start, body.span.end);

        Stmt::new(StmtKind::While { cond, body }, span)
    }

    fn parse_return(&mut self) -> Stmt {
        let start = self.consume(TokenKind::Return);

        let val = if self.check(&TokenKind::Semi) {
            None
        } else {
            Some(self.parse_expr(Precedence::Lowest))
        };

        let end = self.consume(TokenKind::Semi);
        let span = Span::new(start.start, end.end);

        Stmt::new(StmtKind::Return(val), span)
    }

    fn parse_var_decl(&mut self) -> Stmt {
        let start = self.peek().start;
        let is_mutable = if self.peek().kind == TokenKind::Let {
            self.advance();
            true
        } else {
            self.consume(TokenKind::Const);
            false
        };

        let name = self.consume_ident();

        let ty = if self.peek().kind == TokenKind::Colon {
            self.advance();
            Some(self.parse_type())
        } else {
            None
        };

        self.consume(TokenKind::Equal);

        let val = self.parse_expr(Precedence::Lowest);
        let semi = self.consume(TokenKind::Semi);
        let span = Span::new(start, semi.end);

        Stmt::new(
            StmtKind::VarDecl {
                name,
                ty,
                value: val,
                mutable: is_mutable,
            },
            span,
        )
    }

    fn parse_fn_decl(&mut self) -> Stmt {
        let start = self.consume(TokenKind::Fn);
        let name = self.consume_ident();

        self.consume(TokenKind::LeftParen);

        let mut params = Vec::new();

        if !self.check(&TokenKind::RightParen) {
            loop {
                let name = self.consume_ident();
                self.consume(TokenKind::Colon);
                let ty = self.parse_type();

                params.push((name, ty));

                if !self.check(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen);

        let return_type = if self.matches(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };

        let body = Box::new(self.parse_block());
        let span = Span::new(start.start, body.span.end);

        Stmt::new(
            StmtKind::Fn {
                name,
                params,
                return_type,
                body,
            },
            span,
        )
    }

    // -- pratt parsing for expressions --

    pub(crate) fn parse_expr(&mut self, bp: Precedence) -> Expr {
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

            TokenKind::PlusPlus => {
                let rhs = self.parse_expr(Precedence::Prefix);
                let span = Span::merge(span, rhs.span);
                Expr::new(
                    ExprKind::Increment {
                        prefix: true,
                        expr: Box::new(rhs),
                    },
                    span,
                )
            },

            TokenKind::MinusMinus => {
                let rhs = self.parse_expr(Precedence::Prefix);
                let span = Span::merge(span, rhs.span);
                Expr::new(
                    ExprKind::Decrement {
                        prefix: true,
                        expr: Box::new(rhs),
                    },
                    span,
                )
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

            TokenKind::PlusPlus => {
                let span = Span::new(lhs.span.start, op_token.end);
                Expr::new(
                    ExprKind::Increment {
                        prefix: false,
                        expr: Box::new(lhs),
                    },
                    span,
                )
            },

            TokenKind::MinusMinus => {
                let span = Span::new(lhs.span.start, op_token.end);
                Expr::new(
                    ExprKind::Decrement {
                        prefix: false,
                        expr: Box::new(lhs),
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

    fn parse_stmt(input: &str) -> Stmt {
        let lex = Lexer::new(input);
        let tokens: Vec<_> = lex.collect();
        let mut parser = Parser::new(tokens);
        parser.parse()[0].clone()
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

    #[test]
    fn parse_prefix_incr_decr() {
        let expr = parse_expr("++x");
        let expected = ExprKind::Increment {
            prefix: true,
            expr: Box::new(Expr::new(
                ExprKind::Ident("x".to_string()),
                Span::new(2, 3),
            )),
        };
        assert_eq!(expr.node, expected);

        let expr = parse_expr("--x");
        let expected = ExprKind::Decrement {
            prefix: true,
            expr: Box::new(Expr::new(
                ExprKind::Ident("x".to_string()),
                Span::new(2, 3),
            )),
        };
        assert_eq!(expr.node, expected);
    }

    #[test]
    fn parse_postfix_incr_decr() {
        let expr = parse_expr("x++");
        let expected = ExprKind::Increment {
            prefix: false,
            expr: Box::new(Expr::new(
                ExprKind::Ident("x".to_string()),
                Span::new(0, 1),
            )),
        };
        assert_eq!(expr.node, expected);

        let expr = parse_expr("x--");
        let expected = ExprKind::Decrement {
            prefix: false,
            expr: Box::new(Expr::new(
                ExprKind::Ident("x".to_string()),
                Span::new(0, 1),
            )),
        };
        assert_eq!(expr.node, expected);
    }

    #[test]
    fn parse_var_decl() {
        let s = parse_stmt("let x = 5;");
        assert_eq!(
            s.node,
            StmtKind::VarDecl {
                ty: None,
                name: "x".to_string(),
                value: Expr::new(ExprKind::Integer(5), Span::new(8, 9)),
                mutable: true,
            }
        );

        let s = parse_stmt("let x: i16 = 5;");
        assert_eq!(
            s.node,
            StmtKind::VarDecl {
                ty: Some(Type::I16),
                name: "x".to_string(),
                value: Expr::new(ExprKind::Integer(5), Span::new(13, 14)),
                mutable: true,
            }
        );

        let s = parse_stmt("let x: [i16; 3] = 5;");
        assert_eq!(
            s.node,
            StmtKind::VarDecl {
                ty: Some(Type::Array {
                    ty: Box::new(Type::I16),
                    size: 3,
                }),
                name: "x".to_string(),
                value: Expr::new(ExprKind::Integer(5), Span::new(18, 19)),
                mutable: true,
            }
        );

        let s = parse_stmt("let x: *i16 = 5;");
        assert_eq!(
            s.node,
            StmtKind::VarDecl {
                ty: Some(Type::Pointer(Box::new(Type::I16))),
                name: "x".to_string(),
                value: Expr::new(ExprKind::Integer(5), Span::new(14, 15)),
                mutable: true,
            }
        );
    }

    #[test]
    fn parse_if() {
        let stmt = parse_stmt("if (1) something();");
        assert!(matches!(
            stmt.node,
            StmtKind::If {
                else_branch: None,
                ..
            }
        ));

        let stmt = parse_stmt("if (x > 5) { something(); }");
        assert!(matches!(
            stmt.node,
            StmtKind::If {
                else_branch: None,
                ..
            }
        ));

        let stmt = parse_stmt("if (x > 5) something(); else something_else();");
        assert!(matches!(
            stmt.node,
            StmtKind::If {
                else_branch: Some(_),
                ..
            }
        ));

        let stmt = parse_stmt(
            "if (x > 5) { something(); } else { something_else(); }",
        );
        assert!(matches!(
            stmt.node,
            StmtKind::If {
                else_branch: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parse_while() {
        let stmt = parse_stmt("while (x > 5) x--;");
        assert!(matches!(stmt.node, StmtKind::While { .. }));

        let stmt = parse_stmt("while (x > 5) { x--; }");
        assert!(matches!(stmt.node, StmtKind::While { .. }));
    }

    #[test]
    fn parse_fn_decl() {
        let stmt = parse_stmt("fn main() {}");

        let StmtKind::Fn {
            name,
            params,
            return_type,
            ..
        } = stmt.node
        else {
            panic!("not Fn");
        };

        assert_eq!(name, "main".to_string());
        assert_eq!(params.len(), 0);
        assert_eq!(return_type, None);

        let stmt = parse_stmt("fn main(a: i16) {}");
        let StmtKind::Fn { params, .. } = stmt.node else {
            panic!("not Fn");
        };

        assert_eq!(params, vec![("a".to_string(), Type::I16)]);

        let stmt = parse_stmt("fn main(a: i16) -> *i16 {}");
        let StmtKind::Fn { return_type, .. } = stmt.node else {
            panic!("not Fn");
        };

        assert_eq!(return_type, Some(Type::Pointer(Box::new(Type::I16))));
    }
}
