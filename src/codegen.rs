use std::collections::{HashMap, HashSet};

use crate::{
    lexer::token::TokenKind,
    parser::ast::{Expr, ExprKind, NodeId, Span, Stmt, StmtKind, Type},
    visitor::Visitor,
};

#[derive(Debug, Clone)]
pub struct VariableLocation {
    pub offset: i16,
    pub size: u16,
}

pub struct CodeGenerator {
    pub output: String,
    const_directives: String,
    consts: HashSet<String>,
    label_count: usize,
    type_map: HashMap<NodeId, Type>,

    scopes: Vec<HashMap<String, VariableLocation>>,
    current_local_offset: i16,
}

impl CodeGenerator {
    pub fn new(type_map: HashMap<NodeId, Type>) -> Self {
        let prologue = "!start
  cal !main
  jmp !halt
!halt
  jmp !halt\n";

        Self {
            output: prologue.to_string(),
            const_directives: String::new(),
            consts: HashSet::new(),
            label_count: 0,
            type_map,
            scopes: vec![HashMap::new()],
            current_local_offset: 0,
        }
    }

    pub fn into_output(self) -> String {
        format!("{}{}", self.const_directives, self.output)
    }

    pub fn emit(&mut self, instruction: &str) {
        self.output.push_str("    ");
        self.output.push_str(instruction);
        self.output.push('\n');
    }

    pub fn emit_label(&mut self, label: &str) {
        self.output.push_str(label);
        self.output.push('\n');
    }

    pub fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("!{}_{}", prefix, self.label_count);
        self.label_count += 1;
        label
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn insert_var(&mut self, name: String, size: u16) {
        let loc = VariableLocation {
            offset: self.current_local_offset,
            size,
        };
        self.current_local_offset += size as i16;
        self.scopes.last_mut().unwrap().insert(name, loc);
    }

    pub fn lookup_var(&self, name: &str) -> Option<&VariableLocation> {
        for scope in self.scopes.iter().rev() {
            if let Some(loc) = scope.get(name) {
                return Some(loc);
            }
        }
        None
    }

    fn frame_addr(offset: i16) -> String {
        if offset < 0 {
            format!("[rE + {}]", -offset)
        } else {
            format!("[rE - {}]", offset)
        }
    }
}

impl Visitor for CodeGenerator {
    fn visit_integer(&mut self, _span: Span, i: u16) {
        self.emit(&format!("set rA, {}", i));
        self.emit("psh rA");
    }

    fn visit_bool(&mut self, _span: Span, b: bool) {
        self.emit(&format!("set rA, {}", if b { 1 } else { 0 }));
        self.emit("psh rA");
    }

    fn visit_ident(&mut self, _span: Span, ident: &String) {
        if self.consts.contains(ident) {
            self.emit(&format!("set rA, {}", ident));
            self.emit("psh rA");
        } else if let Some(loc) = self.lookup_var(ident) {
            self.emit(&format!("lod rA, {}", Self::frame_addr(loc.offset)));
            self.emit("psh rA");
        }
    }

    fn visit_increment(&mut self, _span: Span, prefix: bool, expr: &Expr) {
        let ExprKind::Ident(ref name) = expr.node else {
            unimplemented!("increment on non-identifier");
        };
        let offset = self
            .lookup_var(name)
            .map(|l| l.offset)
            .expect("undefined variable in increment");

        self.emit(&format!("lod rA, {}", Self::frame_addr(offset)));
        if !prefix {
            self.emit("psh rA");
        }
        self.emit("add rA, 1");
        self.emit(&format!("str {}, rA", Self::frame_addr(offset)));
        if prefix {
            self.emit("psh rA");
        }
    }

    fn visit_decrement(&mut self, _span: Span, prefix: bool, expr: &Expr) {
        let ExprKind::Ident(ref name) = expr.node else {
            unimplemented!("decrement on non-identifier");
        };
        let offset = self
            .lookup_var(name)
            .map(|l| l.offset)
            .expect("undefined variable in decrement");

        self.emit(&format!("lod rA, {}", Self::frame_addr(offset)));
        if !prefix {
            self.emit("psh rA");
        }
        self.emit("sub rA, 1");
        self.emit(&format!("str {}, rA", Self::frame_addr(offset)));
        if prefix {
            self.emit("psh rA");
        }
    }

    fn visit_unary(&mut self, _span: Span, op: &TokenKind, rhs: &Expr) {
        if op == &TokenKind::BitAnd {
            if let ExprKind::Ident(ref name) = rhs.node {
                let offset = self.lookup_var(name).map(|l| l.offset);
                if let Some(offset) = offset {
                    self.emit("set rA, rE");
                    if offset != 0 {
                        self.emit(&format!("sub rA, {}", offset));
                    }
                    self.emit("psh rA");
                } else {
                    unimplemented!("address-of on non-local variable");
                }
            } else {
                unimplemented!("address-of on non-identifier");
            }
            return;
        }

        self.visit_expr(rhs);
        self.emit("pop rA");

        match op {
            TokenKind::Minus => {
                self.emit("neg rA");
            },
            TokenKind::Bang => {
                self.emit("cmp rA, rZ");
                let l_true = self.next_label("not_true");
                let l_end = self.next_label("not_end");
                self.emit(&format!("je {}", l_true));
                self.emit("set rA, 0");
                self.emit(&format!("jmp {}", l_end));
                self.emit_label(&l_true);
                self.emit("set rA, 1");
                self.emit_label(&l_end);
            },
            TokenKind::Star => {
                self.emit("lod rA, [rA]");
            },
            TokenKind::BitNot => {
                self.emit("xor rA, 65535");
            },
            _ => unimplemented!("unary {:?}", op),
        }

        self.emit("psh rA");
    }

    fn visit_binary(
        &mut self,
        _span: Span,
        lhs: &Expr,
        op: &TokenKind,
        rhs: &Expr,
    ) {
        self.visit_expr(lhs);
        self.visit_expr(rhs);

        self.emit("pop rB");
        self.emit("pop rA");

        use TokenKind::*;
        match op {
            Plus => self.emit("add rA, rB"),
            Minus => self.emit("sub rA, rB"),
            Star => self.emit("mpy rA, rB"),
            Slash => self.emit("div rA, rB"),
            Percent => self.emit("mod rA, rB"),
            BitAnd => self.emit("and rA, rB"),
            BitOr => self.emit("or rA, rB"),
            BitXor => self.emit("xor rA, rB"),
            LeftShift => {
                self.emit("neg rB");
                self.emit("shf rA, rB");
            },
            RightShift => self.emit("shf rA, rB"),

            Equality | BangEqual | Gt | Gte | Lt | Lte => {
                self.emit("cmp rA, rB");
                let l_true = self.next_label("cmp_true");
                let l_end = self.next_label("cmp_end");

                let jmp_inst = match op {
                    Equality => "je",
                    BangEqual => "jne",
                    Gt => "jg",
                    Gte => "jge",
                    Lt => "jl",
                    Lte => "jle",
                    _ => unreachable!(),
                };

                self.emit(&format!("{} {}", jmp_inst, l_true));
                self.emit("set rA, 0");
                self.emit(&format!("jmp {}", l_end));
                self.emit_label(&l_true);
                self.emit("set rA, 1");
                self.emit_label(&l_end);
            },

            And => {
                let l_false = self.next_label("and_false");
                let l_end = self.next_label("and_end");
                self.emit("cmp rA, rZ");
                self.emit(&format!("je {}", l_false));
                self.emit("cmp rB, rZ");
                self.emit(&format!("je {}", l_false));
                self.emit("set rA, 1");
                self.emit(&format!("jmp {}", l_end));
                self.emit_label(&l_false);
                self.emit("set rA, 0");
                self.emit_label(&l_end);
            },

            Or => {
                let l_true = self.next_label("or_true");
                let l_end = self.next_label("or_end");
                self.emit("cmp rA, rZ");
                self.emit(&format!("jne {}", l_true));
                self.emit("cmp rB, rZ");
                self.emit(&format!("jne {}", l_true));
                self.emit("set rA, 0");
                self.emit(&format!("jmp {}", l_end));
                self.emit_label(&l_true);
                self.emit("set rA, 1");
                self.emit_label(&l_end);
            },

            _ => unimplemented!("binary {:?}", op),
        }

        self.emit("psh rA");
    }

    fn visit_assignment(&mut self, _span: Span, target: &Expr, value: &Expr) {
        self.visit_expr(value);

        match &target.node {
            ExprKind::Ident(name) => {
                let offset = self.lookup_var(name).map(|l| l.offset);
                if let Some(offset) = offset {
                    self.emit("pop rA");
                    self.emit(&format!("str {}, rA", Self::frame_addr(offset)));
                    self.emit("psh rA");
                }
            },
            ExprKind::Unary { op, rhs } if *op == TokenKind::Star => {
                self.visit_expr(rhs);
                self.emit("pop rB");
                self.emit("pop rA");
                self.emit("str [rB], rA");
                self.emit("psh rA");
            },
            _ => unimplemented!("assignment to non-identifier/dereference"),
        }
    }

    fn visit_var_decl(
        &mut self,
        id: NodeId,
        _span: Span,
        name: &String,
        _ty: &Option<Type>,
        value: &Expr,
        mutable: bool,
    ) {
        if !mutable {
            let val = match &value.node {
                ExprKind::Integer(i) => *i,
                ExprKind::Bool(b) => {
                    if *b {
                        1
                    } else {
                        0
                    }
                },
                _ => todo!(
                    "const initializer must be an integer or bool literal"
                ),
            };
            self.const_directives
                .push_str(&format!(".const {} {}\n", name, val));
            self.consts.insert(name.clone());
            return;
        }

        self.visit_expr(value);
        let ty = self.type_map.get(&id).cloned().unwrap_or(Type::Int);
        self.insert_var(name.clone(), ty.size());
    }

    fn visit_if(
        &mut self,
        _span: Span,
        cond: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) {
        self.visit_expr(cond);
        self.emit("pop rA");
        self.emit("cmp rA, rZ");

        let l_else = self.next_label("if_else");
        let l_end = self.next_label("if_end");

        self.emit(&format!("je {}", l_else));

        self.visit_stmt(then_branch);
        self.emit(&format!("jmp {}", l_end));

        self.emit_label(&l_else);
        if let Some(else_branch) = else_branch {
            self.visit_stmt(else_branch);
        }
        self.emit_label(&l_end);
    }

    fn visit_while(&mut self, _span: Span, cond: &Expr, body: &Stmt) {
        let l_start = self.next_label("while_start");
        let l_end = self.next_label("while_end");

        self.emit_label(&l_start);
        self.visit_expr(cond);
        self.emit("pop rA");
        self.emit("cmp rA, rZ");
        self.emit(&format!("je {}", l_end));

        self.visit_stmt(body);
        self.emit(&format!("jmp {}", l_start));

        self.emit_label(&l_end);
    }

    fn visit_block(&mut self, _span: Span, stmts: &[Stmt]) {
        self.enter_scope();
        let start_offset = self.current_local_offset;

        for stmt in stmts {
            self.visit_stmt(stmt);
        }

        let locals_size = self.current_local_offset - start_offset;
        if locals_size > 0 {
            for _ in 0..locals_size {
                self.emit("pop rB");
            }
        }

        self.current_local_offset = start_offset;
        self.exit_scope();
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
            StmtKind::Expr(expr) => {
                self.visit_expr(expr);
                self.emit("pop rB");
            },
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

    fn visit_fn(
        &mut self,
        _span: Span,
        name: &String,
        params: &[(String, Type)],
        _return_type: &Option<Type>,
        body: &Stmt,
    ) {
        self.emit_label(&format!("!{}", name));
        self.emit("psh rE");
        self.emit("set rE, SP");

        self.enter_scope();
        self.current_local_offset = 0;

        let mut param_iter = params.iter();

        if let Some((p_name, p_ty)) = param_iter.next() {
            self.emit("psh rA");
            self.insert_var(p_name.clone(), p_ty.size());
        }
        if let Some((p_name, p_ty)) = param_iter.next() {
            self.emit("psh rB");
            self.insert_var(p_name.clone(), p_ty.size());
        }
        if let Some((p_name, p_ty)) = param_iter.next() {
            self.emit("psh rC");
            self.insert_var(p_name.clone(), p_ty.size());
        }

        // `cal` pushes the return address (store-then-decrement). After the prologue:
        //   [rE + 1] = saved rE, [rE + 2] = return address, [rE + 3] = arg3, ...
        let mut stack_arg_offset = 3;
        for (p_name, p_ty) in param_iter {
            let loc = VariableLocation {
                offset: -stack_arg_offset,
                size: p_ty.size(),
            };
            stack_arg_offset += p_ty.size() as i16;
            self.scopes.last_mut().unwrap().insert(p_name.clone(), loc);
        }

        self.visit_stmt(body);

        self.emit("set SP, rE");
        self.emit("pop rE");
        self.emit("ret");

        self.exit_scope();
    }

    fn visit_return(&mut self, _span: Span, expr: &Option<Expr>) {
        if let Some(expr) = expr {
            self.visit_expr(expr);
            self.emit("pop rA");
        }

        self.emit("set SP, rE");
        self.emit("pop rE");
        self.emit("ret");
    }

    fn visit_call(&mut self, _span: Span, callee: &Expr, args: &[Expr]) {
        if args.len() > 3 {
            for arg in args.iter().skip(3).rev() {
                self.visit_expr(arg);
            }
        }

        if args.len() > 2 {
            self.visit_expr(&args[2]);
            self.emit("pop rC");
        }
        if args.len() > 1 {
            self.visit_expr(&args[1]);
            self.emit("pop rB");
        }
        if args.len() > 0 {
            self.visit_expr(&args[0]);
            self.emit("pop rA");
        }

        if let ExprKind::Ident(ref name) = callee.node {
            self.emit(&format!("cal !{}", name));
        } else {
            unimplemented!("indirect calls");
        }

        if args.len() > 3 {
            for _ in 3..args.len() {
                self.emit("pop rB");
            }
        }

        self.emit("psh rA");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analysis::{symbol::SymbolResolver, type_checker::TypeChecker},
        lexer::Lexer,
        parser::Parser,
    };

    fn generate_code(input: &str) -> String {
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
        assert!(tc.errors.is_empty(), "Type check errors: {:?}", tc.errors);

        let mut codegen = CodeGenerator::new(tc.type_map);
        for stmt in &ast {
            codegen.visit_stmt(stmt);
        }
        codegen.into_output()
    }

    #[test]
    fn test_codegen_integer() {
        let code = generate_code("fn main() { 42; }");
        assert!(code.contains("set rA, 42"));
        assert!(code.contains("psh rA"));
    }

    #[test]
    fn test_codegen_binary_op() {
        let code = generate_code("fn main() { 5 + 10; }");
        assert!(code.contains("set rA, 5"));
        assert!(code.contains("set rA, 10"));
        assert!(code.contains("add rA, rB"));
    }

    #[test]
    fn test_codegen_variable() {
        let code = generate_code("fn main() { let x = 100; x; }");
        assert!(code.contains("set rA, 100"));
        assert!(code.contains("lod rA, [rE - 0]"));
    }

    #[test]
    fn test_codegen_pointers() {
        let code = generate_code(
            "fn main() { let x: int = 5; let ptr = &x; *ptr = 10; }",
        );
        assert!(code.contains("set rA, rE\n    psh rA"));
        assert!(code.contains("str [rB], rA"));
    }

    #[test]
    fn test_codegen_math_ops() {
        let code = generate_code("fn main() { 5 - 2; 5 * 2; 5 / 2; 5 % 2; }");
        assert!(code.contains("sub rA, rB"));
        assert!(code.contains("mpy rA, rB"));
        assert!(code.contains("div rA, rB"));
        assert!(code.contains("mod rA, rB"));
    }

    #[test]
    fn test_codegen_bitwise_ops() {
        let code = generate_code("fn main() { 5 & 2; 5 | 2; 5 ^ 2; 5 << 2; }");
        assert!(code.contains("and rA, rB"));
        assert!(code.contains("or rA, rB"));
        assert!(code.contains("xor rA, rB"));
        assert!(code.contains("neg rB\n    shf rA, rB"));
    }

    #[test]
    fn test_codegen_right_shift() {
        let code = generate_code("fn main() { 5 >> 2; }");
        assert!(code.contains("shf rA, rB"));
        assert!(!code.contains("neg rB"));
    }

    #[test]
    fn test_codegen_bitwise_not() {
        let code = generate_code("fn main() { ~5; }");
        assert!(code.contains("xor rA, 65535"));
    }

    #[test]
    fn test_codegen_comparisons() {
        let code = generate_code(
            "fn main() { 5 == 2; 5 != 2; 5 < 2; 5 > 2; 5 <= 2; 5 >= 2; }",
        );
        assert!(code.contains("je !cmp_true_"));
        assert!(code.contains("jne !cmp_true_"));
        assert!(code.contains("jl !cmp_true_"));
        assert!(code.contains("jg !cmp_true_"));
        assert!(code.contains("jle !cmp_true_"));
        assert!(code.contains("jge !cmp_true_"));
    }

    #[test]
    fn test_codegen_unary_ops() {
        let code = generate_code("fn main() { -5; !true; }");
        assert!(code.contains("neg rA"));
        assert!(code.contains("je !not_true_"));
    }

    #[test]
    fn test_codegen_logical_and() {
        let code = generate_code("fn main() { true && false; }");
        assert!(code.contains("je !and_false_"));
        assert!(code.contains("!and_false_"));
        assert!(code.contains("!and_end_"));
    }

    #[test]
    fn test_codegen_logical_or() {
        let code = generate_code("fn main() { true || false; }");
        assert!(code.contains("jne !or_true_"));
        assert!(code.contains("!or_true_"));
        assert!(code.contains("!or_end_"));
    }

    #[test]
    fn test_codegen_logical_not() {
        let code = generate_code("fn main() { !true; }");
        assert!(code.contains("je !not_true_"));
        assert!(code.contains("!not_end_"));
    }

    #[test]
    fn test_codegen_complex_expr() {
        let code = generate_code("fn main() { (5 + 2) * 3; }");
        assert!(code.contains("add rA, rB"));
        assert!(code.contains("mpy rA, rB"));
    }

    #[test]
    fn test_codegen_if_else() {
        let code = generate_code("fn main() { if (1) { 2; } else { 3; } }");
        assert!(code.contains("cmp rA, rZ"));
        assert!(code.contains("je !if_else_"));
        assert!(code.contains("jmp !if_end_"));
    }

    #[test]
    fn test_codegen_while() {
        let code = generate_code("fn main() { while (1) { 2; } }");
        assert!(code.contains("!while_start_"));
        assert!(code.contains("cmp rA, rZ"));
        assert!(code.contains("je !while_end_"));
        assert!(code.contains("jmp !while_start_"));
    }

    #[test]
    fn test_codegen_fn_call() {
        let code = generate_code(
            "fn add(a: int, b: int) -> int { return a + b; } fn main() { add(1, 2); }",
        );
        assert!(code.contains("cal !add"));
        assert!(code.contains("psh rE\n    set rE, SP"));
        assert!(code.contains("set SP, rE\n    pop rE\n    ret"));
    }

    #[test]
    fn test_codegen_hex_literal() {
        let code = generate_code("fn main() { 0xFF; 0x1A2B; }");
        assert!(code.contains("set rA, 255"));
        assert!(code.contains("set rA, 6699"));
    }

    #[test]
    fn test_codegen_post_increment() {
        let code = generate_code("fn main() { let x = 0; x++; }");
        assert!(code.contains("lod rA, [rE - 0]"));
        assert!(code.contains("add rA, 1"));
        assert!(code.contains("str [rE - 0], rA"));
    }

    #[test]
    fn test_codegen_pre_increment() {
        let code = generate_code("fn main() { let x = 0; ++x; }");
        assert!(code.contains("add rA, 1"));
        assert!(code.contains("str [rE - 0], rA"));
    }

    #[test]
    fn test_codegen_post_decrement() {
        let code = generate_code("fn main() { let x = 5; x--; }");
        assert!(code.contains("sub rA, 1"));
        assert!(code.contains("str [rE - 0], rA"));
    }

    #[test]
    fn test_codegen_extra_args() {
        let code = generate_code(
            "fn add4(a: int, b: int, c: int, d: int) -> int { return d; } fn main() { add4(1, 2, 3, 4); }",
        );
        assert!(code.contains("lod rA, [rE + 3]"));
    }

    #[test]
    fn test_codegen_const_local() {
        let code = generate_code("fn main() { const MAX = 100; MAX; }");
        assert!(code.starts_with(".const MAX 100\n"));
        assert!(code.contains("set rA, MAX"));
        assert!(!code.contains("psh rA\n    lod rA"));
    }

    #[test]
    fn test_codegen_const_global() {
        let code = generate_code("const PORT = 0x8002; fn main() { PORT; }");
        assert!(code.starts_with(".const PORT 32770\n"));
        assert!(code.contains("set rA, PORT"));
    }

    #[test]
    fn test_codegen_const_bool() {
        let code = generate_code("const FLAG = true; fn main() { FLAG; }");
        assert!(code.starts_with(".const FLAG 1\n"));
        assert!(code.contains("set rA, FLAG"));
    }

    #[test]
    fn test_codegen_const_not_on_stack() {
        let code = generate_code("fn main() { const X = 42; let y = 1; y; }");
        assert!(code.contains(".const X 42"));
        assert!(code.contains("lod rA, [rE - 0]"));
    }

    #[test]
    fn test_codegen_compound_assign() {
        let code = generate_code(
            "fn main() { let x = 1; x += 2; x -= 1; x *= 3; x /= 2; x %= 7; }",
        );
        assert!(code.contains("add rA, rB"));
        assert!(code.contains("sub rA, rB"));
        assert!(code.contains("mpy rA, rB"));
        assert!(code.contains("div rA, rB"));
        assert!(code.contains("mod rA, rB"));
    }
}
