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

    current_fn_return_ty: Option<Type>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(global_scope: &'a HashMap<String, Symbol>) -> Self {
        Self {
            scopes: vec![Scope::new()],
            errors: vec![],
            global_scope,
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
            ExprKind::Integer(_) => Type::I16,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Ident(name) => self
                .lookup(name)
                .cloned()
                .or_else(|| {
                    self.global_scope.get(name).and_then(|sym| {
                        match &sym.kind {
                            SymbolKind::Var { ty, .. } => ty.clone(),
                            SymbolKind::Fn { return_type, .. } => {
                                return_type.clone()
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

            ExprKind::Increment { .. } => Type::I16,
            ExprKind::Decrement { .. } => Type::I16,

            ExprKind::Unary { op, .. } if *op == TokenKind::Minus => Type::I16,
            ExprKind::Unary { op, .. } if *op == TokenKind::Bang => Type::Bool,
            ExprKind::Unary { .. } => {
                return Err(TeenyCompilerError {
                    span: expr.span,
                    kind: TeenyCompilerErrorKind::CannotInferType,
                });
            },

            ExprKind::Binary { lhs, op, rhs } => {
                let lhs_ty = self.infer_type(lhs)?;
                let rhs_ty = self.infer_type(rhs)?;

                if lhs_ty != rhs_ty {
                    return Err(TeenyCompilerError {
                        span: expr.span,
                        kind: TeenyCompilerErrorKind::TypeMismatch(
                            lhs_ty, rhs_ty,
                        ),
                    });
                }

                use TokenKind::*;
                match op {
                    Plus | Minus | Star | Slash | Percent | PlusPlus
                    | MinusMinus | BitAnd | BitOr | BitXor | LeftShift
                    | RightShift => Type::I16,
                    And | Or | Bang | Gt | Gte | Lt | Lte => Type::Bool,
                    _ => panic!("unknown type for {op:?}"),
                }
            },

            ExprKind::Assignment { target, value } => {
                let target_ty = self.infer_type(target)?;
                let value_ty = self.infer_type(value)?;

                if target_ty != value_ty {
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

                if idx_ty != Type::I16 {
                    return Err(TeenyCompilerError {
                        span: index.span,
                        kind: TeenyCompilerErrorKind::TypeMismatch(
                            Type::I16,
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
                self.errors.push(TeenyCompilerError {
                    span,
                    kind: TeenyCompilerErrorKind::TypeMismatch(
                        ty.clone(),
                        inferred,
                    ),
                });
            }

            ty.clone()
        } else {
            inferred
        };

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
            Ok(Type::I16) | Ok(Type::Bool) => {},

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
            Ok(Type::I16) | Ok(Type::Bool) => {},

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
        if let Err(e) = self.infer_type(expr) {
            self.errors.push(e);
        }

        self._visit_expr(expr);
    }
}
