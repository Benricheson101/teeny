pub mod lexer;
pub mod parser;

use lexer::Lexer;
use parser::Parser;

#[cfg(not(coverage))]
fn main() {
    let expr = r"
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
    let ast = prs.parse();

    println!("{ast:#?}");
}
