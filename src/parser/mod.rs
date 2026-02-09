use ast::{Expr, ExprKind, Span, Spanned};

use crate::{
    error::{ParseError, ParseResult},
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

    pub fn parse(&mut self) -> (Vec<Stmt>, Vec<ParseError>) {
        let mut stmts = Vec::new();
        let mut errors: Vec<ParseError> = Vec::new();

        while !self.is_at_end() {
            // stmts.push(self.parse_stmt());
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(e) => {
                    errors.push(e);
                    self.synchronize();
                },
            }
        }

        (stmts, errors)
    }

    fn peek(&self) -> &Token {
        &self.source[self.cur]
    }

    fn prev(&self) -> &Token {
        &self.source[self.cur - 1]
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

    fn check(&self, ty: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }

        &self.peek().kind == ty
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn consume(&mut self, expected: TokenKind) -> ParseResult<Token> {
        if self.check(&expected) {
            Ok(self.advance().clone())
        } else {
            let found = self.peek();
            Err(ParseError {
                message: format!(
                    "Expected token {:?} but found {:?}",
                    expected, found.kind
                ),
                span: Span::new(found.start, found.end),
            })
        }
    }

    fn consume_ident(&mut self) -> ParseResult<String> {
        let token = self.peek();

        match &token.kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            },
            _ => Err(ParseError {
                message: format!("Expected identifier, got {:?}", token),
                span: Span::new(token.start, token.end),
            }),
        }
    }

    fn consume_integer(&mut self) -> ParseResult<i16> {
        let token = self.peek();

        match token.kind {
            TokenKind::Integer(val) => {
                self.advance();
                Ok(val)
            },
            _ => Err(ParseError {
                message: format!("Expected integer, got {:?}", token),
                span: Span::new(token.start, token.end),
            }),
        }
    }

    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            use TokenKind::*;

            if self.prev().kind == Semi {
                return;
            }

            match self.peek().kind {
                Fn | Struct | Let | Const | If | While | Return => return,
                _ => {},
            }
            self.advance();
        }
    }

    fn parse_expr_list(&mut self, term: TokenKind) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();

        if self.peek().kind == term {
            self.advance();
            return Ok(args);
        }

        loop {
            args.push(self.parse_expr(Precedence::Lowest)?);
            if self.peek().kind == TokenKind::Comma {
                self.advance();
            } else {
                break;
            }
        }

        self.consume(term)?;

        Ok(args)
    }

    // -- recursive descent parser for statements --

    pub(crate) fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        let token = self.peek();

        match token.kind {
            TokenKind::Let | TokenKind::Const => self.parse_var_decl(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::LeftBrace => self.parse_block(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Fn => self.parse_fn_decl(),
            TokenKind::Struct => self.parse_struct_decl(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_expr_stmt(&mut self) -> ParseResult<Stmt> {
        let expr = self.parse_expr(Precedence::Lowest)?;
        let semi = self.consume(TokenKind::Semi)?;
        let span = Span::new(expr.span.start, semi.end);

        Ok(Stmt::new(StmtKind::Expr(expr), span))
    }

    /// ```ebnf
    /// type = "i16"
    ///      | "bool"
    ///      | IDENTIFIER
    ///      | "*" type
    ///      | "[" type ";" INTEGER "]" ;
    /// ```
    fn parse_type(&mut self) -> ParseResult<Type> {
        if self.matches(TokenKind::Star) {
            return Ok(Type::Pointer(Box::new(self.parse_type()?)));
        }

        if self.matches(TokenKind::LeftBracket) {
            let elem_type = self.parse_type()?;
            self.consume(TokenKind::Semi)?;

            let size = self.consume_integer()?;

            self.consume(TokenKind::RightBracket)?;

            return Ok(Type::Array {
                ty: Box::new(elem_type),
                size: size as u16,
            });
        }

        match &self.advance().kind {
            TokenKind::I16 => Ok(Type::I16),
            TokenKind::Bool => Ok(Type::Bool),
            TokenKind::Ident(name) => Ok(Type::Struct(name.clone())),
            t @ _ => Err(ParseError {
                message: format!("Expected type, got {:?}", t),
                // span: Span::new(t.start, t.end),
                span: {
                    let prev = self.prev();
                    Span::new(prev.start, prev.end)
                },
            }),
        }
    }

    fn parse_block(&mut self) -> ParseResult<Stmt> {
        let start = self.consume(TokenKind::LeftBrace)?;

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            stmts.push(self.parse_stmt()?);
        }

        let end = self.consume(TokenKind::RightBrace)?;

        let span = Span::new(start.start, end.end);

        Ok(Stmt::new(StmtKind::Block(stmts), span))
    }

    fn parse_if(&mut self) -> ParseResult<Stmt> {
        let start = self.consume(TokenKind::If)?;

        self.consume(TokenKind::LeftParen)?;
        let cond = self.parse_expr(Precedence::Lowest)?;
        self.consume(TokenKind::RightParen)?;

        let then_branch = Box::new(self.parse_stmt()?);

        let else_branch = if self.matches(TokenKind::Else) {
            Some(Box::new(self.parse_stmt()?))
        } else {
            None
        };

        let end_pos = if let Some(ref else_branch) = else_branch {
            else_branch.span.end
        } else {
            then_branch.span.end
        };

        let span = Span::new(start.start, end_pos);

        Ok(Stmt::new(
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            },
            span,
        ))
    }

    fn parse_while(&mut self) -> ParseResult<Stmt> {
        let start = self.consume(TokenKind::While)?;

        self.consume(TokenKind::LeftParen)?;
        let cond = self.parse_expr(Precedence::Lowest)?;
        self.consume(TokenKind::RightParen)?;

        let body = Box::new(self.parse_stmt()?);

        let span = Span::new(start.start, body.span.end);

        Ok(Stmt::new(StmtKind::While { cond, body }, span))
    }

    /// `return = "return" [ expression ] ";" ;`

    fn parse_return(&mut self) -> ParseResult<Stmt> {
        let start = self.consume(TokenKind::Return)?;

        let val = if self.check(&TokenKind::Semi) {
            None
        } else {
            Some(self.parse_expr(Precedence::Lowest)?)
        };

        let end = self.consume(TokenKind::Semi)?;
        let span = Span::new(start.start, end.end);

        Ok(Stmt::new(StmtKind::Return(val), span))
    }

    /// `var_decl = ("let" | "const") IDENTIFIER [ ":" type ] "=" expression ";" ;`
    fn parse_var_decl(&mut self) -> ParseResult<Stmt> {
        let start = self.peek().start;

        let is_mutable = match self.advance().kind {
            TokenKind::Let => true,
            TokenKind::Const => false,
            _ => unreachable!(),
        };

        let name = self.consume_ident()?;

        let ty = if self.peek().kind == TokenKind::Colon {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(TokenKind::Equal)?;

        let val = self.parse_expr(Precedence::Lowest)?;
        let semi = self.consume(TokenKind::Semi)?;

        let span = Span::new(start, semi.end);

        Ok(Stmt::new(
            StmtKind::VarDecl {
                name,
                ty,
                value: val,
                mutable: is_mutable,
            },
            span,
        ))
    }

    /// `fn_decl = "fn" IDENTIFIER "(" [ param_list ] ")" [ "->" type ] block ;`
    fn parse_fn_decl(&mut self) -> ParseResult<Stmt> {
        let start = self.consume(TokenKind::Fn)?;
        let name = self.consume_ident()?;

        self.consume(TokenKind::LeftParen)?;

        let mut params = Vec::new();

        let mut is_first = true;
        if !self.check(&TokenKind::RightParen) {
            loop {
                if is_first
                    && self.peek().kind == TokenKind::Ident("self".to_string())
                {
                    params.push(("self".to_string(), Type::SelfType));
                    is_first = false;
                    self.advance();
                } else {
                    let name = self.consume_ident()?;
                    self.consume(TokenKind::Colon)?;
                    let ty = self.parse_type()?;

                    params.push((name, ty));
                }

                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.consume(TokenKind::RightParen)?;

        let return_type = if self.matches(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = Box::new(self.parse_block()?);
        let span = Span::new(start.start, body.span.end);

        Ok(Stmt::new(
            StmtKind::Fn {
                name,
                params,
                return_type,
                body,
            },
            span,
        ))
    }

    /// ```ebnf
    /// struct_decl = "struct" IDENTIFIER "{" { struct_member } "}" ;
    ///
    /// struct_member = field_decl
    ///               | function_decl ;
    ///
    /// field_decl = IDENTIFIER ":" type [ "," ] ;
    /// ```
    fn parse_struct_decl(&mut self) -> ParseResult<Stmt> {
        let start = self.consume(TokenKind::Struct)?;
        let name = self.consume_ident()?;
        self.consume(TokenKind::LeftBrace)?;

        let mut members = Vec::new();

        while !self.check(&TokenKind::RightBrace) {
            if self.check(&TokenKind::Fn) {
                members.push(self.parse_fn_decl()?);
            } else {
                let start = self.peek().start;

                let field_name = self.consume_ident()?;
                self.consume(TokenKind::Colon)?;
                let field_type = self.parse_type()?;

                let end = self.prev().end;
                let span = Span::new(start, end);

                // FIXME: make it so trailing comma is allowed but commas are otherwise required
                self.matches(TokenKind::Comma);

                members.push(Stmt::new(
                    StmtKind::StructField {
                        name: field_name,
                        ty: field_type,
                    },
                    span,
                ));
            }
        }

        let end = self.consume(TokenKind::RightBrace)?;

        let span = Span::new(start.start, end.end);

        Ok(Stmt::new(StmtKind::Struct { name, members }, span))
    }

    fn parse_struct_init_fields(&mut self) -> ParseResult<Vec<(String, Expr)>> {
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            let name = self.consume_ident()?;
            let val = if self.matches(TokenKind::Colon) {
                self.parse_expr(Precedence::Lowest)?
            } else {
                let prev = self.prev();
                let span = Span::new(prev.start, prev.end);
                Expr::new(ExprKind::Ident(name.clone()), span)
            };

            fields.push((name, val));

            if !self.matches(TokenKind::Comma) {
                break;
            }
        }

        Ok(fields)
    }

    // -- pratt parsing for expressions --

    pub(crate) fn parse_expr(&mut self, bp: Precedence) -> ParseResult<Expr> {
        let mut left = self.parse_prefix()?;

        while bp < Precedence::of(&self.peek().kind) {
            left = self.parse_infix(left)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> ParseResult<Expr> {
        let token = self.advance();
        let span = Span::new(token.start, token.end);

        match &token.kind {
            TokenKind::True => Ok(Expr::new(ExprKind::Bool(true), span)),
            TokenKind::False => Ok(Expr::new(ExprKind::Bool(false), span)),

            TokenKind::Integer(val) => {
                Ok(Expr::new(ExprKind::Integer(*val), span))
            },

            TokenKind::Ident(name) => {
                Ok(Expr::new(ExprKind::Ident(name.clone()), span))
            },

            TokenKind::PlusPlus => {
                let rhs = self.parse_expr(Precedence::Prefix)?;
                let span = Span::merge(span, rhs.span);
                Ok(Expr::new(
                    ExprKind::Increment {
                        prefix: true,
                        expr: Box::new(rhs),
                    },
                    span,
                ))
            },

            TokenKind::MinusMinus => {
                let rhs = self.parse_expr(Precedence::Prefix)?;
                let span = Span::merge(span, rhs.span);
                Ok(Expr::new(
                    ExprKind::Decrement {
                        prefix: true,
                        expr: Box::new(rhs),
                    },
                    span,
                ))
            },

            TokenKind::LeftBracket => {
                let elems = self.parse_expr_list(TokenKind::RightBracket)?;
                Ok(Expr::new(ExprKind::Array(elems), span))
            },

            tk @ (TokenKind::Minus | TokenKind::Bang) => {
                let tk = tk.clone();
                let right = self.parse_expr(Precedence::Prefix)?;
                let span = Span::merge(span, right.span);

                Ok(Spanned::new(
                    ExprKind::Unary {
                        op: tk,
                        rhs: Box::new(right),
                    },
                    span,
                ))
            },

            TokenKind::LeftParen => {
                let expr = self.parse_expr(Precedence::Lowest)?;

                let next = self.peek();
                if next.kind == TokenKind::RightParen {
                    self.advance();
                } else {
                    return Err(ParseError {
                        message: "Expected ')'".to_string(),
                        span,
                    });
                }

                Ok(expr)
            },

            _ => Err(ParseError {
                message: format!("Unexpected token: {token:?}"),
                span,
            }),
        }
    }

    fn parse_infix(&mut self, lhs: Expr) -> ParseResult<Expr> {
        let op_token = self.advance().clone();

        match op_token.kind {
            TokenKind::Equal
            | TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual
            | TokenKind::SlashEqual => {
                let value = self.parse_expr(Precedence::Lowest)?;
                if !matches!(lhs.node, ExprKind::Ident(_)) {
                    return Err(ParseError {
                        message: format!(
                            "Assigning to a non-variable: {lhs:?}"
                        ),
                        span: Span::new(op_token.start, op_token.end),
                    });
                }

                let span = Span::merge(lhs.span, value.span);
                Ok(Expr::new(
                    ExprKind::Assignment {
                        target: Box::new(lhs),
                        value: Box::new(value),
                    },
                    span,
                ))
            },

            TokenKind::PlusPlus => {
                let span = Span::new(lhs.span.start, op_token.end);
                Ok(Expr::new(
                    ExprKind::Increment {
                        prefix: false,
                        expr: Box::new(lhs),
                    },
                    span,
                ))
            },

            TokenKind::MinusMinus => {
                let span = Span::new(lhs.span.start, op_token.end);
                Ok(Expr::new(
                    ExprKind::Decrement {
                        prefix: false,
                        expr: Box::new(lhs),
                    },
                    span,
                ))
            },

            TokenKind::LeftParen => {
                let args = self.parse_expr_list(TokenKind::RightParen)?;
                let span =
                    Span::new(lhs.span.start, self.source[self.cur - 1].end);

                Ok(Expr::new(
                    ExprKind::Call {
                        callee: Box::new(lhs),
                        args,
                    },
                    span,
                ))
            },

            TokenKind::LeftBracket => {
                let index = self.parse_expr(Precedence::Lowest)?;
                self.consume(TokenKind::RightBracket)?;
                let span = Span::merge(lhs.span, index.span);

                Ok(Expr::new(
                    ExprKind::Subscript {
                        array: Box::new(lhs),
                        index: Box::new(index),
                    },
                    span,
                ))
            },

            TokenKind::Dot => {
                let name = self.consume_ident()?;
                let span =
                    Span::new(lhs.span.start, self.source[self.cur - 1].end);

                Ok(Expr::new(
                    ExprKind::MemberAccess {
                        object: Box::new(lhs),
                        name,
                    },
                    span,
                ))
            },

            TokenKind::LeftBrace => {
                match lhs.node {
                    ExprKind::Ident(_) | ExprKind::MemberAccess { .. } => {},
                    _ => {
                        return Err(ParseError {
                            message: "Struct init must follow a type name"
                                .to_string(),
                            span: lhs.span,
                        });
                    },
                }

                let fields = self.parse_struct_init_fields()?;
                let end_token = self.consume(TokenKind::RightBrace)?;
                let span = Span::new(lhs.span.start, end_token.end);

                Ok(Expr::new(
                    ExprKind::StructInit {
                        name: Box::new(lhs),
                        fields,
                    },
                    span,
                ))
            },

            _ => {
                let op_bp = Precedence::of(&op_token.kind);
                let rhs = self.parse_expr(op_bp)?;
                let span = Span::merge(lhs.span, rhs.span);

                Ok(Expr::new(
                    ExprKind::Binary {
                        lhs: Box::new(lhs),
                        op: op_token.kind,
                        rhs: Box::new(rhs),
                    },
                    span,
                ))
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
        parser.parse_expr(Precedence::Lowest).unwrap()
    }

    fn parse_stmt(input: &str) -> Stmt {
        let lex = Lexer::new(input);
        let tokens: Vec<_> = lex.collect();
        let mut parser = Parser::new(tokens);
        parser.parse().0[0].clone()
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
    fn parse_struct_init() {
        let expr = parse_expr("Point { x: 5, y: 0 }");
        let expected = ExprKind::StructInit {
            name: Box::new(Expr::new(
                ExprKind::Ident("Point".to_string()),
                Span::new(0, 5),
            )),
            fields: vec![
                (
                    "x".to_string(),
                    Expr::new(ExprKind::Integer(5), Span::new(11, 12)),
                ),
                (
                    "y".to_string(),
                    Expr::new(ExprKind::Integer(0), Span::new(17, 18)),
                ),
            ],
        };
        assert_eq!(expr.node, expected);

        let expr = parse_expr("Point { x, y: 3 }");
        let expected = ExprKind::StructInit {
            name: Box::new(Expr::new(
                ExprKind::Ident("Point".to_string()),
                Span::new(0, 5),
            )),
            fields: vec![
                (
                    "x".to_string(),
                    Expr::new(
                        ExprKind::Ident("x".to_string()),
                        Span::new(8, 9),
                    ),
                ),
                (
                    "y".to_string(),
                    Expr::new(ExprKind::Integer(3), Span::new(14, 15)),
                ),
            ],
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

        let s = parse_stmt("const x: [i16; 3] = 5;");
        assert_eq!(
            s.node,
            StmtKind::VarDecl {
                ty: Some(Type::Array {
                    ty: Box::new(Type::I16),
                    size: 3,
                }),
                name: "x".to_string(),
                value: Expr::new(ExprKind::Integer(5), Span::new(20, 21)),
                mutable: false,
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

        let stmt = parse_stmt("fn main(a: i16, b: i16) {}");
        let StmtKind::Fn { params, .. } = stmt.node else {
            panic!("not Fn");
        };

        assert_eq!(
            params,
            vec![("a".to_string(), Type::I16), ("b".to_string(), Type::I16)]
        );

        let stmt = parse_stmt("fn main(a: i16) -> *i16 {}");
        let StmtKind::Fn { return_type, .. } = stmt.node else {
            panic!("not Fn");
        };

        assert_eq!(return_type, Some(Type::Pointer(Box::new(Type::I16))));
    }

    #[test]
    fn parse_struct_decl() {
        let stmt = parse_stmt(
            "
            struct User {
                name: string,
                id: i16,

                fn get_id(self) {
                    return self.id;
                }
            }
        ",
        );

        let StmtKind::Struct { name, members } = stmt.node else {
            panic!("not struct");
        };

        assert_eq!(name, "User".to_string());
        assert_eq!(members.len(), 3);
    }

    #[test]
    fn parse_errors_syncronize() {
        // let
    }
}
