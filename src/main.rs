pub mod analysis;
pub mod cli;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod visitor;

use std::{fs, process};

use analysis::{symbol::SymbolResolver, type_checker::TypeChecker};
use clap::Parser;
use codegen::CodeGenerator;
use lexer::Lexer;
use visitor::Visitor;

use crate::error::print_errors;

// #[cfg(not(coverage))]
fn main() {
    let cli = cli::Cli::parse();

    let expr = fs::read_to_string(&cli.in_file).unwrap();

    // println!("{expr}");

    let lex = Lexer::new(&expr);
    let tokens: Vec<_> = lex.collect();
    // println!("{:#?}", &tokens);

    let mut prs = parser::Parser::new(tokens);
    let (ast, errors) = prs.parse();

    // println!("{ast:#?}");

    let filename = cli.in_file.to_str().unwrap_or("main.tny");

    if !errors.is_empty() {
        print_errors(&expr, filename, &errors);
        process::exit(1);
    }

    let mut sr = SymbolResolver::new();
    if let Err(errors) = sr.check(&ast) {
        print_errors(&expr, filename, &errors);
        process::exit(1);
    } else {
        let global_scope = sr.global_scope();
        let mut tc = TypeChecker::new(global_scope);
        for stmt in &ast {
            tc.visit_stmt(stmt);
        }

        print_errors(&expr, filename, &tc.errors);

        if !tc.errors.is_empty() {
            process::exit(1);
        }

        let mut codegen = CodeGenerator::new(tc.type_map);
        for stmt in &ast {
            codegen.visit_stmt(stmt);
        }

        let output = codegen.into_output();
        if let Some(out_file) = cli.out_file {
            fs::write(out_file, output).unwrap();
        } else {
            print!("{}", output);
        }
    }
}
