use std::collections::HashMap;

use crate::{
    analysis::symbol::{Symbol, SymbolKind},
    error::{TeenyCompilerError, TeenyCompilerErrorKind},
    lexer::token::TokenKind,
    parser::ast::{Expr, ExprKind, Span, Stmt, Type},
    visitor::Visitor,
};

type Scope = HashMap<String, Type>;

pub struct TypeChecker<'a> {
    scopes: Vec<Scope>,
    global_scope: &'a HashMap<String, Symbol>,
    pub errors: Vec<TeenyCompilerError>,
    pub type_map: HashMap<crate::parser::ast::NodeId, Type>,

    current_fn_return_ty: Option<Type>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(global_scope: &'a HashMap<String, Symbol>) -> Self {
        Self {
            scopes: vec![Scope::new()],
            errors: vec![],
            global_scope,
            type_map: HashMap::new(),
            current_fn_return_ty: None,
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn lookup(&self, name: &String) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }

        None
    }

    fn insert(&mut self, name: String, ty: Type) {
        self.scopes
            .last_mut()
            .expect("always has at least one scope")
            .insert(name, ty);
    }

    fn infer_type(&self, expr: &Expr) -> Result<Type, TeenyCompilerError> {
        let ty: Type = match &expr.node {
            ExprKind::Integer(_) => Type::Int,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Ident(name) => self
                .lookup(name)
                .cloned()
                .or_else(|| {
                    self.global_scope.get(name).and_then(|sym| {
                        match &sym.kind {
                            SymbolKind::Var { ty, .. } => ty.clone(),
                            SymbolKind::Fn { return_type, .. } => {
                                Some(return_type.clone().unwrap_or(Type::Void))
                            },
                            SymbolKind::Struct { .. } => {
                                Some(Type::Struct(name.clone()))
                            },
                        }
                    })
                })
                .ok_or_else(|| TeenyCompilerError {
                    span: expr.span,
                    kind: TeenyCompilerErrorKind::CannotInferType,
                })?,

            ExprKind::Array(items) if !items.is_empty() => {
                let first_ty = self.infer_type(&items[0])?;

                for item in items.iter().skip(1) {
                    let item_ty = self.infer_type(item)?;
                    if item_ty != first_ty {
                        return Err(TeenyCompilerError {
                            span: item.span,
                            kind: TeenyCompilerErrorKind::TypeMismatch(
                                first_ty, item_ty,
                            ),
                        });
                    }
                }

                first_ty
            },
            ExprKind::Array(_) => {
                return Err(TeenyCompilerError {
                    span: expr.span,
                    kind: TeenyCompilerErrorKind::CannotInferType,
                });
            },

            ExprKind::Increment { .. } => Type::Int,
            ExprKind::Decrement { .. } => Type::Int,

            ExprKind::Unary { op, rhs } => {
                let rhs_ty = self.infer_type(rhs)?;
                match op {
                    TokenKind::Minus => {
                        if rhs_ty == Type::Int {
                            rhs_ty
                        } else {
                            return Err(TeenyCompilerError {
                                span: expr.span,
                                kind: TeenyCompilerErrorKind::TypeMismatch(
                                    Type::Int,
                                    rhs_ty,
                                ),
                            });
                        }
                    },
                    TokenKind::Bang => {
                        if rhs_ty == Type::Bool {
                            Type::Bool
                        } else {
                            return Err(TeenyCompilerError {
                                span: expr.span,
                                kind: TeenyCompilerErrorKind::TypeMismatch(
                                    Type::Bool,
                                    rhs_ty,
                                ),
                            });
                        }
                    },
                    TokenKind::Star => {
                        if let Type::Pointer(inner) = rhs_ty {
                            *inner
                        } else {
                            // Not a pointer
                            return Err(TeenyCompilerError {
                                span: expr.span,
                                kind: TeenyCompilerErrorKind::CannotInferType, // FIXME: better error
                            });
                        }
                    },
                    TokenKind::BitAnd => Type::Pointer(Box::new(rhs_ty)),
                    TokenKind::BitNot => {
                        if rhs_ty == Type::Int {
                            Type::Int
                        } else {
                            return Err(TeenyCompilerError {
                                span: expr.span,
                                kind: TeenyCompilerErrorKind::TypeMismatch(
                                    Type::Int,
                                    rhs_ty,
                                ),
                            });
                        }
                    },
                    _ => {
                        return Err(TeenyCompilerError {
                            span: expr.span,
                            kind: TeenyCompilerErrorKind::CannotInferType,
                        });
                    },
                }
            },

            ExprKind::Binary { lhs, op, rhs } => {
                let lhs_ty = self.infer_type(lhs)?;
                let rhs_ty = self.infer_type(rhs)?;

                // Bool and Int are the same 16-bit word at runtime.
                // Allow mixing them in comparisons (e.g. `bool_val == 1`).
                let is_int_bool_cmp = matches!(
                    (&lhs_ty, &rhs_ty),
                    (Type::Bool, Type::Int) | (Type::Int, Type::Bool)
                ) && matches!(
                    op,
                    TokenKind::Equality
                        | TokenKind::BangEqual
                        | TokenKind::Lt
                        | TokenKind::Lte
                        | TokenKind::Gt
                        | TokenKind::Gte
                );

                if lhs_ty != rhs_ty && !is_int_bool_cmp {
                    if let Type::Pointer(_) = lhs_ty {
                        if rhs_ty == Type::Int {
                            if *op == TokenKind::Plus || *op == TokenKind::Minus
                            {
                                return Ok(lhs_ty);
                            }
                        }
                    }
                    return Err(TeenyCompilerError {
                        span: expr.span,
                        kind: TeenyCompilerErrorKind::TypeMismatch(
                            lhs_ty, rhs_ty,
                        ),
                    });
                }

                let resolved_ty = lhs_ty;

                use TokenKind::*;
                match op {
                    Plus | Minus | Star | Slash | Percent | PlusPlus
                    | MinusMinus | BitAnd | BitOr | BitXor | LeftShift
                    | RightShift => resolved_ty,
                    And | Or | Bang | Gt | Gte | Lt | Lte => Type::Bool,
                    Equality | BangEqual => Type::Bool,
                    _ => panic!("unknown type for {op:?}"),
                }
            },

            ExprKind::Assignment { target, value } => {
                let target_ty = self.infer_type(target)?;
                let value_ty = self.infer_type(value)?;

                if target_ty != value_ty {
                    // Allow assigning a raw integer address to a pointer
                    if matches!(target_ty, Type::Pointer(_))
                        && value_ty == Type::Int
                    {
                        return Ok(target_ty);
                    }
                    return Err(TeenyCompilerError {
                        span: expr.span,
                        kind: TeenyCompilerErrorKind::TypeMismatch(
                            target_ty, value_ty,
                        ),
                    });
                }

                value_ty
            },

            ExprKind::Call { callee, args } => {
                let ExprKind::Ident(fn_name) = &callee.node else {
                    return Err(TeenyCompilerError {
                        span: expr.span,
                        // FIXME: use proper error type
                        kind: TeenyCompilerErrorKind::SyntaxError,
                    });
                };

                let Some(Symbol {
                    kind:
                        SymbolKind::Fn {
                            params,
                            return_type,
                        },
                    ..
                }) = self.global_scope.get(fn_name)
                else {
                    // I don't think this ever gets reached
                    return Err(TeenyCompilerError {
                        span: callee.span,
                        kind: TeenyCompilerErrorKind::IdentNotDefined(
                            fn_name.to_string(),
                        ),
                    });
                };

                if params.len() != args.len() {
                    return Err(TeenyCompilerError {
                        span: expr.span,
                        kind: TeenyCompilerErrorKind::ParamCountMismatch(
                            params.len(),
                            args.len(),
                        ),
                    });
                }

                for (i, (_, param_ty)) in params.iter().enumerate() {
                    let arg_ty = self.infer_type(&args[i])?;
                    if param_ty != &arg_ty {
                        // TODO: see if this can return multiple errors instead of just one
                        return Err(TeenyCompilerError {
                            span: args[i].span,
                            kind: TeenyCompilerErrorKind::TypeMismatch(
                                param_ty.clone(),
                                arg_ty,
                            ),
                        });
                    }
                }

                return_type.clone().unwrap_or(Type::Void)
            },

            ExprKind::Subscript { array, index } => {
                let arr_ty = self.infer_type(array)?;
                let idx_ty = self.infer_type(index)?;

                if idx_ty != Type::Int {
                    return Err(TeenyCompilerError {
                        span: index.span,
                        kind: TeenyCompilerErrorKind::TypeMismatch(
                            Type::Int,
                            idx_ty,
                        ),
                    });
                }

                match arr_ty {
                    Type::Array { ty, .. } => *ty,
                    _ => {
                        return Err(TeenyCompilerError {
                            span: expr.span,
                            kind: TeenyCompilerErrorKind::CannotInferType,
                        });
                    },
                }
            },

            ExprKind::MemberAccess { object, name } => {
                todo!()
            },

            ExprKind::StaticAccess { target, member } => todo!(),

            ExprKind::StructInit { name, fields } => {
                todo!()
            },
        };

        Ok(ty)
    }
}

impl<'a> Visitor for TypeChecker<'a> {
    fn visit_var_decl(
        &mut self,
        id: crate::parser::ast::NodeId,
        span: Span,
        name: &String,
        ty: &Option<Type>,
        value: &Expr,
        _mutable: bool,
    ) {
        let inferred = match self.infer_type(value) {
            Ok(ty) => ty,
            Err(e) => {
                self.errors.push(e);
                Type::Error
            },
        };

        let resolved_ty = if let Some(ty) = ty {
            if ty != &inferred && inferred != Type::Error {
                // Allow assigning a raw integer address to a pointer
                let is_int_to_ptr =
                    matches!(ty, Type::Pointer(_)) && inferred == Type::Int;
                if !is_int_to_ptr {
                    self.errors.push(TeenyCompilerError {
                        span,
                        kind: TeenyCompilerErrorKind::TypeMismatch(
                            ty.clone(),
                            inferred,
                        ),
                    });
                }
            }

            ty.clone()
        } else {
            inferred
        };

        self.type_map.insert(id, resolved_ty.clone());

        // let ty = ty.clone().unwrap_or_else(|| self.infer_type(value));
        self.insert(name.clone(), resolved_ty);
        self.visit_expr(value);
    }

    fn visit_fn(
        &mut self,
        _span: Span,
        _name: &String,
        params: &[(String, Type)],
        return_type: &Option<Type>,
        body: &Stmt,
    ) {
        self.enter_scope();
        let old_ret_ty = self.current_fn_return_ty.clone();
        self.current_fn_return_ty = return_type.clone();

        for (param_name, param_ty) in params {
            self.insert(param_name.clone(), param_ty.clone());
        }

        self.visit_stmt(body);

        self.current_fn_return_ty = old_ret_ty;
        self.exit_scope();
    }

    fn visit_block(&mut self, _span: Span, stmts: &[Stmt]) {
        self.enter_scope();

        for stmt in stmts {
            self.visit_stmt(stmt);
        }

        self.exit_scope();
    }

    fn visit_return(&mut self, span: Span, expr: &Option<Expr>) {
        if expr.is_none() && !self.current_fn_return_ty.is_none() {
            self.errors.push(TeenyCompilerError {
                span,
                kind: TeenyCompilerErrorKind::TypeMismatch(
                    self.current_fn_return_ty.clone().unwrap(),
                    Type::Void,
                ),
            });
        }

        let expected = self.current_fn_return_ty.clone().unwrap_or(Type::Void);

        match (expected, expr) {
            (Type::Void, None) => {},

            (Type::Void, Some(expr)) => {
                let ty = match self.infer_type(expr) {
                    Ok(ty) => ty,
                    Err(e) => {
                        self.errors.push(e);
                        Type::Error
                    },
                };
                self.errors.push(TeenyCompilerError {
                    span,
                    kind: TeenyCompilerErrorKind::TypeMismatch(
                        Type::Void,
                        // FIXME: this should never print Type::Error
                        ty,
                    ),
                });
            },

            (expected, None) => {
                self.errors.push(TeenyCompilerError {
                    span,
                    kind: TeenyCompilerErrorKind::TypeMismatch(
                        expected,
                        Type::Void,
                    ),
                });
            },

            (expected, Some(expr)) => {
                let ty = match self.infer_type(expr) {
                    Ok(ty) => ty,
                    Err(e) => {
                        self.errors.push(e);
                        Type::Error
                    },
                };

                if ty != expected && ty != Type::Error {
                    self.errors.push(TeenyCompilerError {
                        span,
                        kind: TeenyCompilerErrorKind::TypeMismatch(
                            expected, ty,
                        ),
                    });
                }
            },
        }
    }

    fn visit_if(
        &mut self,
        span: Span,
        cond: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) {
        // TODO: is it ok to compare on ints in teenyat?
        match self.infer_type(cond) {
            Ok(Type::Int) | Ok(Type::Bool) => {},

            Ok(ty) => self.errors.push(TeenyCompilerError {
                span,
                kind: TeenyCompilerErrorKind::TypeMismatch(Type::Bool, ty),
            }),

            Err(e) => {
                self.errors.push(e);
            },
        }

        self.visit_expr(cond);
        self.visit_stmt(then_branch);
        if let Some(else_branch) = else_branch {
            self.visit_stmt(else_branch);
        }
    }

    fn visit_while(&mut self, span: Span, cond: &Expr, body: &Stmt) {
        match self.infer_type(cond) {
            Ok(Type::Int) | Ok(Type::Bool) => {},

            Ok(ty) => self.errors.push(TeenyCompilerError {
                span,
                kind: TeenyCompilerErrorKind::TypeMismatch(Type::Bool, ty),
            }),

            Err(e) => {
                self.errors.push(e);
            },
        }

        self.visit_expr(cond);
        self.visit_stmt(body);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match self.infer_type(expr) {
            Ok(ty) => {
                self.type_map.insert(expr.id, ty);
            },
            Err(e) => {
                self.errors.push(e);
            },
        }

        self._visit_expr(expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analysis::symbol::SymbolResolver,
        lexer::Lexer,
        parser::Parser,
    };

    fn check_types(input: &str) -> Vec<TeenyCompilerError> {
        let lex = Lexer::new(input);
        let tokens: Vec<_> = lex.collect();
        let mut parser = Parser::new(tokens);
        let (ast, errors) = parser.parse();
        assert!(errors.is_empty(), "Parser errors: {:?}", errors);

        let mut sr = SymbolResolver::new();
        sr.check(&ast).expect("Symbol resolution failed");

        let global_scope = sr.global_scope();
        let mut tc = TypeChecker::new(global_scope);
        for stmt in &ast {
            tc.visit_stmt(stmt);
        }

        tc.errors.clone()
    }

    fn ok(input: &str) {
        let errors = check_types(input);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    fn err(input: &str) {
        let errors = check_types(input);
        assert!(!errors.is_empty(), "Expected type error but got none");
    }

    #[test]
    fn var_decl_int_annotated() {
        ok("fn main() { let x: int = 5; }");
    }

    #[test]
    fn var_decl_int_inferred() {
        ok("fn main() { let x = 5; }");
    }

    #[test]
    fn var_decl_bool_annotated() {
        ok("fn main() { let x: bool = true; }");
    }

    #[test]
    fn var_decl_bool_inferred() {
        ok("fn main() { let x = false; }");
    }

    #[test]
    fn var_decl_uses_earlier_local() {
        ok("fn main() { let x: int = 5; let y: int = x + 10; }");
    }

    #[test]
    fn var_decl_type_mismatch_int_bool() {
        err("fn main() { let x: int = true; }");
    }

    #[test]
    fn var_decl_type_mismatch_bool_int() {
        err("fn main() { let x: bool = 5; }");
    }

    #[test]
    fn pointer_address_of() {
        ok("fn main() { let x: int = 5; let ptr: *int = &x; }");
    }

    #[test]
    fn pointer_deref() {
        ok(
            "fn main() { let x: int = 5; let ptr: *int = &x; let val: int = *ptr; }",
        );
    }

    #[test]
    fn pointer_arithmetic() {
        ok("fn main() { let x: int = 5; let ptr = &x; let next = ptr + 1; }");
    }

    #[test]
    fn pointer_from_integer_literal() {
        ok("fn main() { let p: *int = 0x9000; }");
    }

    #[test]
    fn pointer_assign_integer_address() {
        ok("fn main() { let x: int = 5; let p: *int = &x; p = 0x8000; }");
    }

    #[test]
    fn pointer_type_mismatch() {
        err("fn main() { let x: int = 5; let ptr: *bool = &x; }");
    }

    #[test]
    fn deref_non_pointer() {
        err("fn main() { let x: int = 5; let y = *x; }");
    }

    #[test]
    fn unary_minus_int() {
        ok("fn main() { let x: int = -5; }");
    }

    #[test]
    fn unary_minus_on_bool_errors() {
        err("fn main() { let x = -true; }");
    }

    #[test]
    fn unary_not_bool() {
        ok("fn main() { let x: bool = !true; }");
    }

    #[test]
    fn unary_not_on_int_errors() {
        err("fn main() { let x = !5; }");
    }

    #[test]
    fn binary_arithmetic() {
        ok(
            "fn main() { let a: int = 3 + 4; let b: int = 3 - 4; let c: int = 3 * 4; let d: int = 8 / 2; let e: int = 7 % 3; }",
        );
    }

    #[test]
    fn binary_bitwise() {
        ok(
            "fn main() { let a: int = 3 & 4; let b: int = 3 | 4; let c: int = 3 ^ 4; let d: int = 1 << 2; }",
        );
    }

    #[test]
    fn binary_comparisons_return_bool() {
        ok(
            "fn main() { let a: bool = 1 == 1; let b: bool = 1 != 2; let c: bool = 1 < 2; let d: bool = 2 > 1; let e: bool = 1 <= 1; let f: bool = 1 >= 1; }",
        );
    }

    #[test]
    fn binary_logical_and_returns_bool() {
        ok("fn main() { let a: bool = true && false; }");
    }

    #[test]
    fn binary_logical_or_returns_bool() {
        ok("fn main() { let a: bool = false || true; }");
    }

    #[test]
    fn binary_logical_type_mismatch() {
        err("fn main() { let x = true && 5; }");
    }

    #[test]
    fn binary_type_mismatch() {
        err("fn main() { let x = 5 + true; }");
    }

    #[test]
    fn bool_compared_to_int() {
        ok("fn main() { let x = 5 == 5; if (x == 1) { } }");
    }

    #[test]
    fn if_bool_literal_condition() {
        ok("fn main() { if (true) { } }");
    }

    #[test]
    fn if_int_literal_condition() {
        ok("fn main() { if (1) { } }");
    }

    #[test]
    fn if_equality_condition() {
        ok("fn main() { let x: int = 5; let y: int = 5; if (x == y) { } }");
    }

    #[test]
    fn if_comparison_condition() {
        ok("fn main() { let x: int = 5; if (x > 0) { } }");
    }

    #[test]
    fn if_else() {
        ok("fn main() { let x: int = 1; if (x == 1) { } else { } }");
    }

    #[test]
    fn while_bool_condition() {
        ok("fn main() { while (true) { } }");
    }

    #[test]
    fn while_int_condition() {
        ok("fn main() { let x: int = 10; while (x) { } }");
    }

    #[test]
    fn while_comparison_condition() {
        ok("fn main() { let x: int = 5; while (x > 0) { } }");
    }

    #[test]
    fn post_increment_type() {
        ok("fn main() { let x: int = 0; x++; }");
    }

    #[test]
    fn pre_increment_type() {
        ok("fn main() { let x: int = 0; ++x; }");
    }

    #[test]
    fn post_decrement_type() {
        ok("fn main() { let x: int = 5; x--; }");
    }

    #[test]
    fn compound_assign_plus() {
        ok("fn main() { let x: int = 1; x += 2; }");
    }

    #[test]
    fn compound_assign_minus() {
        ok("fn main() { let x: int = 5; x -= 2; }");
    }

    #[test]
    fn compound_assign_star() {
        ok("fn main() { let x: int = 3; x *= 4; }");
    }

    #[test]
    fn compound_assign_slash() {
        ok("fn main() { let x: int = 8; x /= 2; }");
    }

    #[test]
    fn fn_void_no_return() {
        ok("fn foo() { } fn main() { foo(); }");
    }

    #[test]
    fn fn_with_return_type() {
        ok("fn answer() -> int { return 42; } fn main() { answer(); }");
    }

    #[test]
    fn fn_with_params() {
        ok(
            "fn add(a: int, b: int) -> int { return a + b; } fn main() { add(1, 2); }",
        );
    }

    #[test]
    fn fn_return_type_mismatch() {
        err("fn foo() -> int { return true; }");
    }

    #[test]
    fn fn_return_value_from_void() {
        err("fn foo() { return 5; }");
    }

    #[test]
    fn fn_missing_return_value() {
        err("fn foo() -> int { return; }");
    }

    #[test]
    fn fn_call_wrong_arg_type() {
        err("fn foo(x: int) { } fn main() { foo(true); }");
    }

    #[test]
    fn fn_call_too_many_args() {
        err("fn foo(x: int) { } fn main() { foo(1, 2); }");
    }

    #[test]
    fn fn_call_too_few_args() {
        err("fn foo(x: int, y: int) { } fn main() { foo(1); }");
    }

    #[test]
    fn variable_shadow_in_inner_scope() {
        ok("fn main() { let x: int = 1; if (1) { let x: bool = true; } }");
    }

    #[test]
    fn variable_used_after_block() {
        ok("fn main() { let x: int = 5; if (1) { } let y: int = x + 1; }");
    }
}
