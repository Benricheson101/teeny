pub mod analysis;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod visitor;

use analysis::{symbol::SymbolResolver, type_checker::TypeChecker};
use lexer::Lexer;
use parser::Parser;
use visitor::Visitor;

#[cfg(not(coverage))]
fn main() {
    use crate::error::print_errors;

    let expr = r"
        struct Point {
            x: i16,
            y: i16,

            fn add(self, other: Point) -> Point {
                return Point { x: self.x + other.x, y: self.y + other.y };
            }
        }

        fn add(a: i16, b: i16) -> i16 {
            return a + b;
        }

        fn main() {
            let a = Point { x: 0, y: 0 };
            let b = Point { x: 5, y: 10 };
            let c = a.add(b);

            let x = 1;
            let y = 10;
            let z = add(x, y);
        }
    ";

    let expr = r"
        struct Point {
            x: i16,
            y: i16,
        }

        fn add(a: i16, b: i16) -> i16 {
            return a;
        }

        fn main() {
            let x = 5;
            add(x, x);
        }
    ";

    println!("{expr}");

    let lex = Lexer::new(expr);
    let tokens: Vec<_> = lex.collect();
    println!("{:#?}", &tokens);

    let mut prs = Parser::new(tokens);
    let (ast, errors) = prs.parse();

    println!("{ast:#?}");

    print_errors(expr, "main.tny", &errors);

    let mut sr = SymbolResolver::new();
    if let Err(errors) = sr.check(&ast) {
        print_errors(expr, "main.tny", &errors);
    }

    let global_scope = sr.global_scope();
    let mut tc = TypeChecker::new(global_scope);
    for stmt in ast {
        tc.visit_stmt(&stmt);
    }

    print_errors(expr, "main.tny", &tc.errors);
}
