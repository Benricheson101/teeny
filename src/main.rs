pub mod lexer;
pub mod parser;

use lexer::Lexer;
use parser::{Parser, precedence::Precedence};

#[cfg(not(coverage))]
fn main() {
    let expr = "mystruct.arr[x((x + 1) * 3) * 2]";
    // let expr = r"
    //     fn main() {
    //         let x = 5;
    //         let y = 10;
    //         let z = x + y;
    //         print(z);
    //     }
    // ";
    println!("{expr}");

    let lex = Lexer::new(expr);
    let tokens: Vec<_> = lex.collect();
    println!("{:#?}", &tokens);

    let mut prs = Parser::new(tokens);
    let ast = prs.parse_expr(Precedence::Lowest);

    println!("{ast:#?}");
}
