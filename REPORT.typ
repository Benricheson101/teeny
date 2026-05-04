#set page(paper: "us-letter", margin: 1in, numbering: "1")
#set text(font: "New Computer Modern", size: 12pt)
#set par(justify: true, leading: 0.65em, spacing: 1.5em)
#set heading(numbering: none)

#let snippet(
  caption: none,
  body
) = {
  show raw.where(block: true): it => box(
    fill: luma(240),
    inset: (x: 5pt, y: 5pt),
    width: 1fr,
    outset: (y: 5pt),
    radius: 2pt,
    text(font: "Berkeley Mono", size: 10pt, it)
  )

  show figure.caption.where(kind: "snippet"): set text(size: 10pt)

  figure(
    caption: caption,
    supplement: "Snippet",
    kind: "snippet",
    align(left, body),
  )
}

#show heading: it => {
  set text(size: 14pt, weight: "bold")
  block(above: 1.5em, below: 0.6em, it)
}
#align(center)[
  #text(size: 16pt, weight: "bold")[Teeny: A Compiler for the TeenyAT] \
  Ben Richeson \
  #text(size: 12pt)[CS Capstone — SUNY Polytechnic Institute \ May 2026]
]

#v(1em)

= Introduction
Teeny is a compiled programming language targeting the TeenyAT, a 16-bit virtual embedded microcontroller. The language syntax is loosely inspired by that of Rust and C.

#snippet(
  caption: "Bubble sort implemented in Teeny",
)[
```rs
fn bubble_sort(arr_start: *int, len: int) {
    let i = 0;
    while (i < len - 1) {
        let j = 0;
        while (j < len - i - 1) {
            let cur = *(arr_start + j);
            let next = *(arr_start + j + 1);

            if (cur > next) {
                *(arr_start + j) = next;
                *(arr_start + j + 1) = cur;
            }
            j++;
        }
        i++;
    }
}
```
]

The goal of the project was to build a compiler from scratch, following the traditional pipeline of lexical analysis, parsing, static analysis, and code generation. Planned language features included variables, functions, mathematical expressions, control flow, and loops. Stretch goals for the project additionally included struct and array types, optimizations, and file imports.

The teenyc compiler is entirely hand-written in Rust. The only third-party dependency used is an error reporting library called Miette for nicely formatted error messages. I chose to use Rust over another language like Python or C++ because many Rust language features lend themselves kindly to the internals of a compiler.

The project took a test-driven approach. Each feature of the language has at least one corresponding test. These tests verify expected behavior against actual behavior of the code and help prevent regressions down the line.

= How it Works
The teenyc compiler follows the traditional pipeline of a compiler: lexical analysis, parsing, static analysis, code generation. Splitting it apart into distinct stages helped keep the project organized. The output from one stage became the input to the next stage.

Lexical analysis takes in a string and turns it into a stream of semantically-meaningful tokens. It extracts keywords, identifiers, integers, literals, and other symbols used in the code. Rust’s enum type is perfect for this because enum variants can hold associated data.

#snippet(caption: "TokenKind enum")[
```rs
enum TokenKind {
    Let,
    Equal,
    Plus,
    Semi,
    Ident(String),
    Integer(u16),
}

assert_eq!(
    lex("let x = 5 + 5;"),
    vec![Let, Ident("x"), Equal, Integer(5), Plus, Integer(5), Semi]
);
```
]

The next stage of the pipeline is parsing. The parser takes in a list of tokens and transforms it into an abstract syntax tree, a tree representation of the structure of the code. To do this, I used two different algorithms: recursive descent, and Pratt.

Before I began, I found the idea of parsing a flat list of tokens into a deep tree to be daunting. I was pleasantly surprised to learn about an algorithm called recursive descent that makes this both trivial and elegant for a simple language like Teeny. The idea behind recursive descent is that the language syntax is broken down into a set of recursive functions that each parse a non-terminal part of the grammar. As a result, the parsing code closely resembles the language grammar. For example, both an `if` statement and a `while` loop share similar syntax: they begin with a keyword, followed by a parenthesized expression, followed by a block statement. With recursive descent, this would likely result in the following functions: `parse_statement`, `parse_while`, `parse_if`, `parse_expression`, and `parse_block`.

#snippet(caption: "Parser code for parsing a `while` statement")[
```rs
/// while_stmt = "while" "(" expr ")" stmt ;
fn parse_while(&mut self) -> ParseResult<Stmt> {
    let start = self.consume(TokenKind::While)?;

    self.consume(TokenKind::LeftParen)?;
    let cond = self.parse_expr(Precedence::Lowest)?;
    self.consume(TokenKind::RightParen)?;

    let body = Box::new(self.parse_stmt()?);
    let span = Span::new(start.start, body.span.end);
    Ok(Stmt::new(StmtKind::While { cond, body }, span))
}
```
]

One shortcoming of recursive descent is that it’s impossible to convey operator precedence without having many nearly identical parsing functions. My solution to this was to use recursive descent only to parse statements and use a second type of parser for expressions: the Pratt parser. Unlike recursive descent, Pratt is more complicated to implement initially but conveys operator precedence in a very simple way. Operators are assigned a numerical binding power, where the higher binding power takes precedence over a lower one. Teeny’s operator precedences match those of C and many other popular languages.

#snippet(caption: "Token precedence assignments")[
```rs
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Precedence {
    Lowest,
    Assignment,
    // <snip>
    Sum,
    Product,
    Call,
}

impl Precedence {
    pub fn of(kind: &TokenKind) -> Self {
        use TokenKind::*;

        match kind {
            Equal | PlusEqual | MinusEqual => Self::Assignment,
            // <snip>
            Plus | Minus => Self::Sum,
            Star | Slash | Percent => Self::Product,
            _ => Self::Lowest,
        }
    }
}
```
]

After parsing is static analysis. Teenyc performs two different kinds of analyses: symbol resolution, and type checking. The symbol resolver traverses the syntax tree and checks the validity of all function calls and variables with respect to their scopes.

As the name suggests, the type checker checks the validity of data types used in the language. It does this through a combination of explicitly defined types and type inference. The type checker traverses the syntax tree to its leaves and propagates the types of values. An error is thrown by the type checker if the resolved type of a symbol doesn't match the expected type. Additionally, the type checker stage builds a key-value mapping of symbols and their data types.

The final stage of the pipeline is code generation. By this point, it can be safely assumed that the code is syntactically and semantically valid. The code generator once again traverses the syntax tree, and outputs TeenyAT assembly for each tree node it visits.

To simplify the code generation process, teenyc uses a stack machine approach. Not having to track register usage is a key advantage of this approach. A stack machine pushes all values onto the stack and only pops them off into a register when they are needed. Once an operation is completed (e.g. `add`), the result is pushed back onto the stack. In the context of code generation, this means only two registers are ever needed and can be hard-coded in the generated assembly code.

Teenyc uses a callee-saved calling convention; a called function is responsible for ensuring registers remain unchanged after returning. The only exception to this is that the function's returned value is placed in register `rA`. Function arguments are placed in registers `rA`-`rC`, then spilled onto the stack. Register `rE` is used to track stack offsets for local variables.

#snippet(caption: "Sample Teeny function")[
```rs
fn add(a: int, b: int) -> int {
    return a + b;
}
```
]

#snippet(caption: [Annotated teenyc output for `add` function])[
```asm
!add
    ; back up the stack pointer in rE
    psh rE
    set rE, SP

    ; rA: first argument (`a`)
    psh rA
    ; rB: second argument (`b`)
    psh rB

    ; load `a` from memory into the top of the stack
    lod rA, [rE - 0]
    psh rA

    ; load `b` from memory into the top of the stack
    lod rA, [rE - 1]
    psh rA

    ; pop two operands from the top of the stack
    pop rB
    pop rA

    ; perform add operation
    add rA, rB
    ; push result to stack
    psh rA

    ; pop return value into rA
    pop rA

    ; clear memory used within the function
    set SP, rE
    ; restore stack pointer
    pop rE

    ; return
    ret
```
]

= How it Went
My initial goals for the compiler were a bit overambitious, but the final product is still a fully functioning compiler for the TeenyAT covering most of the available operations. Ultimately, implementing structs ended up becoming too complex with the current architecture of the compiler, but arrays can still be implemented using pointers.

Initially I had hoped to implement some basic optimizations, like dead code removal, constant folding, and a register allocator. These three optimizations would make a world of difference to a program running on a system like the TeenyAT. While a stack machine is simple to implement, it has a minimum of five memory operations for every binary operation, which hurts the performance of a compiled program significantly. Instead, a register allocator would analyze the code and dynamically assign long-lived values to all five available registers. This means that, at minimum, a binary expression could involve zero memory operations instead of the mandatory five used by a stack machine.

= What I Learned
Through my studies at SUNY Poly, and on my own, I've used dozens of different languages on several different platforms. Doing this project gave me a new kind of respect for what goes into a programming language and a compiler. It’s a combination of all different areas of computer science wrapped up into one project. There are the obvious ones like programming language theory and design, and low-level computer architecture, but also some less obvious areas like the dozens of data structures and algorithms used, and the finite state machine that is the lexer.

Building teenyc showed me how much computer science theory I've learned. If I were to continue the project, I would research and implement optimizations targeting higher performance output code, as well as add more features to the language like structs and arrays.
