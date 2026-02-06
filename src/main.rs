pub mod error;
pub mod lexer;
pub mod parser;

use lexer::Lexer;
use parser::Parser;

#[cfg(not(coverage))]
fn main() {
    let expr = r"
        struct User {
            name: String,
            id: i16,

            fn get_id(self) -> i16 {
                return self.id;
            }
        }

        fn add(a: i16, b: i16) -> i16 {
            return a + b;
        }

        fn main() {
            let x = 0;
            let y = 10;
            let z = add(a, b);
        }
    ";
    println!("{expr}");

    let lex = Lexer::new(expr);
    let tokens: Vec<_> = lex.collect();
    println!("{:#?}", &tokens);

    let mut prs = Parser::new(tokens);
    let (ast, errors) = prs.parse();

    println!("{ast:#?}");
    println!("{errors:#?}");
}
