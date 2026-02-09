pub mod error;
pub mod lexer;
pub mod parser;

use lexer::Lexer;
use miette::NamedSource;
use parser::Parser;

#[cfg(not(coverage))]
fn main() {
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

    let expr = "ivt::register {balls};";

    println!("{expr}");

    let lex = Lexer::new(expr);
    let tokens: Vec<_> = lex.collect();
    println!("{:#?}", &tokens);

    let mut prs = Parser::new(tokens);
    let (ast, errors) = prs.parse();

    println!("{ast:#?}");

    for err in errors {
        let source = NamedSource::new("main.tny", expr.to_string()); //.read_span(&span, 1, 1).unwrap();
        let err = miette::Report::new(err).with_source_code(source);
        eprintln!("{:?}", err);
    }
}
