use crate::{
    lexer::token::TokenKind,
    parser::ast::{Expr, ExprKind, NodeId, Span, Stmt, StmtKind, Type},
};

pub trait Visitor {
    fn _visit_expr(&mut self, expr: &Expr) {
        let span = expr.span;
        match &expr.node {
            ExprKind::Integer(i) => self.visit_integer(span, *i),
            ExprKind::Bool(b) => self.visit_bool(span, *b),
            ExprKind::Ident(ident) => self.visit_ident(span, ident),
            ExprKind::Array(exprs) => self.visit_array(span, exprs),
            ExprKind::Increment { prefix, expr } => {
                self.visit_increment(span, *prefix, expr)
            },
            ExprKind::Decrement { prefix, expr } => {
                self.visit_decrement(span, *prefix, expr)
            },
            ExprKind::Unary { op, rhs } => self.visit_unary(span, op, rhs),
            ExprKind::Binary { lhs, op, rhs } => {
                self.visit_binary(span, lhs, op, rhs)
            },
            ExprKind::Assignment { target, value } => {
                self.visit_assignment(span, target, value)
            },
            ExprKind::Call { callee, args } => {
                self.visit_call(span, callee, args)
            },
            ExprKind::Subscript { array, index } => {
                self.visit_subscript(span, array, index)
            },
            ExprKind::MemberAccess { object, name } => {
                self.visit_member_access(span, object, name)
            },
            ExprKind::StaticAccess { target, member } => {
                self.visit_static_access(span, target, member)
            },
            ExprKind::StructInit { name, fields } => {
                self.visit_struct_init(span, name, fields)
            },
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        self._visit_expr(expr);
    }

    fn visit_integer(&mut self, _span: Span, _i: i16) {
    }

    fn visit_bool(&mut self, _span: Span, _b: bool) {
    }

    fn visit_ident(&mut self, _span: Span, _ident: &String) {
    }

    fn visit_array(&mut self, _span: Span, exprs: &[Expr]) {
        for expr in exprs {
            self.visit_expr(expr);
        }
    }

    fn visit_increment(&mut self, _span: Span, _prefix: bool, expr: &Expr) {
        self.visit_expr(expr);
    }

    fn visit_decrement(&mut self, _span: Span, _prefix: bool, expr: &Expr) {
        self.visit_expr(expr);
    }

    fn visit_unary(&mut self, _span: Span, _op: &TokenKind, rhs: &Expr) {
        self.visit_expr(rhs);
    }

    fn visit_binary(
        &mut self,
        _span: Span,
        lhs: &Expr,
        _op: &TokenKind,
        rhs: &Expr,
    ) {
        self.visit_expr(lhs);
        self.visit_expr(rhs);
    }

    fn visit_assignment(&mut self, _span: Span, target: &Expr, value: &Expr) {
        self.visit_expr(target);
        self.visit_expr(value);
    }

    fn visit_call(&mut self, _span: Span, callee: &Expr, args: &[Expr]) {
        self.visit_expr(callee);
        for arg in args {
            self.visit_expr(arg);
        }
    }

    fn visit_subscript(&mut self, _span: Span, array: &Expr, index: &Expr) {
        self.visit_expr(array);
        self.visit_expr(index);
    }

    fn visit_member_access(
        &mut self,
        _span: Span,
        object: &Expr,
        _name: &String,
    ) {
        self.visit_expr(object);
    }

    fn visit_static_access(
        &mut self,
        _span: Span,
        target: &Expr,
        _member: &String,
    ) {
        self.visit_expr(target);
    }

    fn visit_struct_init(
        &mut self,
        _span: Span,
        name: &Expr,
        fields: &[(String, Expr)],
    ) {
        self.visit_expr(name);
        for (_name, field) in fields {
            self.visit_expr(field);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        let span = stmt.span;
        let id = stmt.id;
        match &stmt.node {
            StmtKind::VarDecl {
                name,
                ty,
                value,
                mutable,
            } => self.visit_var_decl(id, span, name, ty, value, *mutable),
            StmtKind::Expr(expr) => self.visit_expr(expr),
            StmtKind::Block(stmts) => self.visit_block(span, stmts),
            StmtKind::Return(expr) => self.visit_return(span, expr),
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            } => self.visit_if(span, cond, then_branch, else_branch.as_deref()),
            StmtKind::While { cond, body } => {
                self.visit_while(span, cond, body)
            },
            StmtKind::Fn {
                name,
                params,
                return_type,
                body,
            } => self.visit_fn(span, name, params, return_type, body),
            StmtKind::Struct { name, members } => {
                self.visit_struct(span, name, members)
            },
            StmtKind::StructField { name, ty } => {
                self.visit_struct_field(span, name, ty)
            },
        }
    }

    fn visit_var_decl(
        &mut self,
        _id: NodeId,
        _span: Span,
        _name: &String,
        _ty: &Option<Type>,
        value: &Expr,
        _mutable: bool,
    ) {
        self.visit_expr(value);
    }

    fn visit_block(&mut self, _span: Span, stmts: &[Stmt]) {
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_return(&mut self, _span: Span, expr: &Option<Expr>) {
        if let Some(expr) = expr {
            self.visit_expr(expr);
        }
    }

    fn visit_if(
        &mut self,
        _span: Span,
        cond: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) {
        self.visit_expr(cond);
        self.visit_stmt(then_branch);
        if let Some(else_branch) = else_branch {
            self.visit_stmt(else_branch);
        }
    }

    fn visit_while(&mut self, _span: Span, cond: &Expr, body: &Stmt) {
        self.visit_expr(cond);
        self.visit_stmt(body);
    }

    fn visit_fn(
        &mut self,
        _span: Span,
        _name: &String,
        _params: &[(String, Type)],
        _return_type: &Option<Type>,
        body: &Stmt,
    ) {
        self.visit_stmt(body);
    }

    fn visit_struct(&mut self, _span: Span, _name: &String, members: &[Stmt]) {
        for member in members {
            self.visit_stmt(member);
        }
    }

    fn visit_struct_field(&mut self, _span: Span, _name: &String, _ty: &Type) {
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        lexer::token::TokenKind,
        parser::ast::{ExprKind, Span, Spanned, Stmt, StmtKind, Type},
        visitor::Visitor,
    };

    struct CounterVisitor {
        integer_count: usize,
        bool_count: usize,
        ident_count: usize,
    }

    impl CounterVisitor {
        fn new() -> Self {
            Self {
                integer_count: 0,
                bool_count: 0,
                ident_count: 0,
            }
        }
    }

    impl Visitor for CounterVisitor {
        fn visit_integer(&mut self, _span: Span, _i: i16) {
            self.integer_count += 1;
        }

        fn visit_bool(&mut self, _span: Span, _b: bool) {
            self.bool_count += 1;
        }

        fn visit_ident(&mut self, _span: Span, _ident: &String) {
            self.ident_count += 1;
        }
    }

    macro_rules! test_expr {
        ($expr:expr $(, $field:ident = $val:expr)* $(,)?) => {{
            let mut visitor = CounterVisitor::new();
            visitor.visit_expr(&$expr);
            $(assert_eq!(visitor.$field, $val);)*
        }}
    }

    macro_rules! test_stmt {
        ($stmt:expr $(, $field:ident = $val:expr)* $(,)?) => {{
            let mut visitor = CounterVisitor::new();
            visitor.visit_stmt(&$stmt);
            $(assert_eq!(visitor.$field, $val);)*
        }}
    }

    fn spanned<T>(node: T) -> Spanned<T> {
        Spanned::new(node, Span::new(0, 0))
    }

    fn boxed<T>(node: T) -> Box<Spanned<T>> {
        Box::new(spanned(node))
    }

    #[test]
    fn test_visit_array() {
        let arr = spanned(ExprKind::Array(vec![
            spanned(ExprKind::Integer(0)),
            spanned(ExprKind::Integer(1)),
            spanned(ExprKind::Integer(2)),
        ]));

        test_expr!(arr, integer_count = 3);
    }

    #[test]
    fn test_visit_increment() {
        let expr = spanned(ExprKind::Increment {
            prefix: true,
            expr: boxed(ExprKind::Ident("i".to_string())),
        });

        test_expr!(expr, ident_count = 1);
    }

    #[test]
    fn test_visit_decrement() {
        let expr = spanned(ExprKind::Decrement {
            prefix: true,
            expr: boxed(ExprKind::Ident("i".to_string())),
        });

        test_expr!(expr, ident_count = 1);
    }

    #[test]
    fn test_visit_unary() {
        let expr = spanned(ExprKind::Unary {
            op: TokenKind::Bang,
            rhs: boxed(ExprKind::Ident("i".to_string())),
        });

        test_expr!(expr, ident_count = 1);
    }

    #[test]
    fn test_visit_binary() {
        let expr = spanned(ExprKind::Binary {
            lhs: boxed(ExprKind::Ident("n".to_string())),
            op: TokenKind::Plus,
            rhs: boxed(ExprKind::Integer(5)),
        });

        test_expr!(expr, ident_count = 1, integer_count = 1)
    }

    #[test]
    fn test_visit_assignment() {
        let expr = spanned(ExprKind::Assignment {
            target: boxed(ExprKind::Ident("i".to_string())),
            value: boxed(ExprKind::Integer(5)),
        });

        test_expr!(expr, ident_count = 1, integer_count = 1)
    }

    #[test]
    fn test_visit_call() {
        let expr = spanned(ExprKind::Call {
            callee: boxed(ExprKind::Ident("foo".to_string())),
            args: vec![
                spanned(ExprKind::Integer(1)),
                spanned(ExprKind::Integer(2)),
            ],
        });

        test_expr!(expr, ident_count = 1, integer_count = 2)
    }

    #[test]
    fn test_visit_subscript() {
        let expr = spanned(ExprKind::Subscript {
            array: boxed(ExprKind::Ident("arr".to_string())),
            index: boxed(ExprKind::Ident("i".to_string())),
        });

        test_expr!(expr, ident_count = 2)
    }

    #[test]
    fn test_visit_member_access() {
        let expr = spanned(ExprKind::MemberAccess {
            object: boxed(ExprKind::Ident("obj".to_string())),
            name: "x".to_string(),
        });

        test_expr!(expr, ident_count = 1)
    }

    #[test]
    fn test_visit_static_access() {
        let expr = spanned(ExprKind::StaticAccess {
            target: boxed(ExprKind::Ident("ivt".to_string())),
            member: "register".to_string(),
        });

        test_expr!(expr, ident_count = 1)
    }

    #[test]
    fn test_visit_struct_init() {
        let expr = spanned(ExprKind::StructInit {
            name: boxed(ExprKind::Ident("Point".to_string())),
            fields: vec![
                ("x".to_string(), spanned(ExprKind::Integer(1))),
                ("y".to_string(), spanned(ExprKind::Integer(2))),
            ],
        });

        test_expr!(expr, ident_count = 1, integer_count = 2)
    }

    #[test]
    fn test_visit_var_decl() {
        let stmt = spanned(StmtKind::VarDecl {
            name: "x".to_string(),
            ty: Some(Type::I16),
            value: spanned(ExprKind::Integer(42)),
            mutable: true,
        });

        test_stmt!(stmt, integer_count = 1)
    }

    #[test]
    fn test_visit_expr_stmt() {
        let stmt = spanned(StmtKind::Expr(spanned(ExprKind::Integer(5))));

        test_stmt!(stmt, integer_count = 1)
    }

    #[test]
    fn test_visit_block() {
        let stmt = spanned(StmtKind::Block(vec![
            spanned(StmtKind::Expr(spanned(ExprKind::Integer(1)))),
            spanned(StmtKind::Expr(spanned(ExprKind::Integer(2)))),
            spanned(StmtKind::Expr(spanned(ExprKind::Integer(3)))),
        ]));

        test_stmt!(stmt, integer_count = 3)
    }

    #[test]
    fn test_visit_return_with_value() {
        let stmt =
            spanned(StmtKind::Return(Some(spanned(ExprKind::Integer(42)))));

        test_stmt!(stmt, integer_count = 1)
    }

    #[test]
    fn test_visit_return_no_value() {
        let stmt = spanned(StmtKind::Return(None));

        test_stmt!(stmt, integer_count = 0, bool_count = 0, ident_count = 0)
    }

    #[test]
    fn test_visit_if_no_else() {
        let then_body: Stmt =
            spanned(StmtKind::Expr(spanned(ExprKind::Integer(1))));
        let stmt = spanned(StmtKind::If {
            cond: spanned(ExprKind::Bool(true)),
            then_branch: Box::new(then_body),
            else_branch: None,
        });

        test_stmt!(stmt, bool_count = 1, integer_count = 1)
    }

    #[test]
    fn test_visit_if_with_else() {
        let then_body: Stmt =
            spanned(StmtKind::Expr(spanned(ExprKind::Integer(1))));
        let else_body: Stmt =
            spanned(StmtKind::Expr(spanned(ExprKind::Integer(2))));
        let stmt = spanned(StmtKind::If {
            cond: spanned(ExprKind::Bool(true)),
            then_branch: Box::new(then_body),
            else_branch: Some(Box::new(else_body)),
        });

        test_stmt!(stmt, bool_count = 1, integer_count = 2)
    }

    #[test]
    fn test_visit_while() {
        let body: Stmt = spanned(StmtKind::Expr(spanned(ExprKind::Integer(1))));
        let stmt = spanned(StmtKind::While {
            cond: spanned(ExprKind::Bool(true)),
            body: Box::new(body),
        });

        test_stmt!(stmt, bool_count = 1, integer_count = 1)
    }

    #[test]
    fn test_visit_fn() {
        let return_stmt: Stmt =
            spanned(StmtKind::Return(Some(spanned(ExprKind::Integer(42)))));
        let body: Stmt = spanned(StmtKind::Block(vec![return_stmt]));
        let stmt = spanned(StmtKind::Fn {
            name: "foo".to_string(),
            params: vec![("x".to_string(), Type::I16)],
            return_type: Some(Type::I16),
            body: Box::new(body),
        });

        test_stmt!(stmt, integer_count = 1)
    }

    #[test]
    fn test_visit_struct() {
        let stmt = spanned(StmtKind::Struct {
            name: "Point".to_string(),
            members: vec![
                spanned(StmtKind::StructField {
                    name: "x".to_string(),
                    ty: Type::I16,
                }),
                spanned(StmtKind::StructField {
                    name: "y".to_string(),
                    ty: Type::I16,
                }),
            ],
        });

        test_stmt!(stmt, integer_count = 0, bool_count = 0, ident_count = 0)
    }
}
