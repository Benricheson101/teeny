pub mod lexer;
pub mod parser;

use lexer::Lexer;
use parser::{Parser, precedence::Precedence};

#[cfg(not(coverage))]
fn main() {
    let expr = "5 * (2 + 3)";

    let lex = Lexer::new(expr);
    let tokens: Vec<_> = lex.collect();
    println!("{:#?}", &tokens);

    let mut prs = Parser::new(tokens);
    let ast = prs.parse_expr(Precedence::Lowest);

    println!("{:#?}", ast);
}
