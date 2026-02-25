use std::collections::HashMap;

use crate::{
    error::{TeenyCompilerError, TeenyCompilerErrorKind},
    parser::ast::{Expr, Span, Stmt, Type},
    visitor::Visitor,
};

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Var {
        ty: Option<Type>,
        mutable: bool,
    },

    Fn {
        params: Vec<(String, Type)>,
        return_type: Option<Type>,
    },

    Struct {
        fields: Vec<(String, Type)>,
    },
}

pub type Scope = HashMap<String, Symbol>;

pub struct SymbolResolver {
    scopes: Vec<Scope>,
    pub errors: Vec<TeenyCompilerError>,
}

impl SymbolResolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()],
            errors: vec![],
        }
    }

    pub fn check(
        &mut self,
        ast: &[Stmt],
    ) -> Result<(), Vec<TeenyCompilerError>> {
        for stmt in ast {
            self.visit_stmt(stmt);
        }

        if self.errors.is_empty() {
            return Ok(());
        }

        Err(self.errors.clone())
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn exit_scope(&mut self) {
        assert_ne!(self.scopes.len(), 1, "cannot exit global scope");
        self.scopes.pop();
    }

    pub fn lookup(&self, name: &String) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }

        None
    }

    pub fn insert(&mut self, sym: Symbol) {
        self.scopes
            .last_mut()
            .expect("always has at least one scope")
            .insert(sym.name.clone(), sym);
    }
}

impl Visitor for SymbolResolver {
    fn visit_var_decl(
        &mut self,
        _span: Span,
        name: &String,
        ty: &Option<Type>,
        value: &Expr,
        mutable: bool,
    ) {
        self.insert(Symbol {
            name: name.clone(),
            kind: SymbolKind::Var {
                ty: ty.clone(),
                mutable,
            },
        });

        self.visit_expr(value);
    }

    fn visit_fn(
        &mut self,
        _span: Span,
        name: &String,
        params: &[(String, Type)],
        return_type: &Option<Type>,
        body: &Stmt,
    ) {
        self.insert(Symbol {
            name: name.clone(),
            kind: SymbolKind::Fn {
                params: params.to_vec().clone(),
                return_type: return_type.clone(),
            },
        });
        self.enter_scope();

        for (param_name, param_ty) in params {
            self.insert(Symbol {
                name: param_name.clone(),
                kind: SymbolKind::Var {
                    ty: Some(param_ty.clone()),
                    mutable: false,
                },
            })
        }

        self.visit_stmt(body);

        self.exit_scope();
    }

    fn visit_struct(&mut self, _span: Span, name: &String, members: &[Stmt]) {
        self.insert(Symbol {
            name: name.clone(),
            kind: SymbolKind::Struct {
                // fields are checked during the type checking pass
                fields: vec![],
            },
        });

        self.enter_scope();
        for member in members {
            self.visit_stmt(member);
        }
        self.exit_scope();
    }

    fn visit_block(&mut self, _span: Span, stmts: &[Stmt]) {
        self.enter_scope();
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
        self.exit_scope();
    }

    fn visit_ident(&mut self, span: Span, ident: &String) {
        if let None = self.lookup(ident) {
            self.errors.push(TeenyCompilerError {
                kind: TeenyCompilerErrorKind::IdentNotDefined(ident.clone()),
                span: span.clone(),
            });
        }
    }
}
