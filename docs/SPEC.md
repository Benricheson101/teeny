# Teeny Language Specification

## Program Structure

**Extension:** `.tny`
**Entrypoint:** `main()`

A Teeny program is a flat list of top-level declarations (functions and constants).
Execution begins at `main()`. All functions must be declared at the global scope.

The syntax loosely matches that of Rust and C.

---

## Memory Model

- All values are **16-bit integers**.
- `bool` is represented as an integer: `0` is false, any other value is true.
- Pointers are `int` values that hold a memory address.
- All memory is either statically allocated (`.const` directives) or stack-allocated (local variables). There is no heap.

---

## Lexical Rules

### Comments

```rs
// single-line comment
```

### Identifiers

Identifiers start with a letter or `_`, followed by any combination of letters, digits, `_`, and `$`.

```
identifier = [a-zA-Z_] [a-zA-Z0-9_$]*
```

### Integer Literals

Decimal and hexadecimal integer literals are supported. All values are stored as 16-bit unsigned and interpreted as signed at runtime.

```rs
255       // decimal
0xFF      // hexadecimal (case-insensitive prefix)
0xBEEF
```

### Boolean Literals

```rs
true
false
```

---

## Types

| Type  | Size    | Description                    |
|-------|---------|--------------------------------|
| `int` | 1 word  | 16-bit signed integer          |
| `bool`| 1 word  | Boolean (0 = false, else true) |
| `*T`  | 1 word  | Pointer to a value of type `T` |

Types can be nested: `*int`, `*bool`.

---

## Variables

`let` declares a mutable local variable. `const` declares an immutable named constant whose initializer must be an integer or boolean literal. Constants are emitted as `.const` directives and do not consume stack space.

```rs
let x = 10;
let y: int = 20;

const PORT: *int = 0x8002;
const MAX: int = 100;
const FLAG: bool = true;
```

Type annotations (`: type`) are optional.

Variables are scoped to their enclosing block. A `let` variable must be initialized at the point of declaration.

---

## Operators

### Arithmetic

| Operator | Meaning        |
|----------|----------------|
| `+`      | Addition        |
| `-`      | Subtraction     |
| `*`      | Multiplication  |
| `/`      | Division        |
| `%`      | Modulo          |

### Bitwise

| Operator | Meaning        |
|----------|----------------|
| `&`      | Bitwise AND     |
| `\|`     | Bitwise OR      |
| `^`      | Bitwise XOR     |
| `~`      | Bitwise NOT     |
| `<<`     | Left shift      |
| `>>`     | Right shift     |

### Logical

| Operator | Meaning     |
|----------|-------------|
| `&&`     | Logical AND |
| `\|\|`   | Logical OR  |
| `!`      | Logical NOT |

Logical AND and OR short-circuit. `!` requires a `bool` operand; `~` requires an `int` operand.

### Comparison

| Operator | Meaning                  |
|----------|--------------------------|
| `==`     | Equal                    |
| `!=`     | Not equal                |
| `<`      | Less than                |
| `<=`     | Less than or equal       |
| `>`      | Greater than             |
| `>=`     | Greater than or equal    |

Comparisons produce a `bool` result.

### Assignment

Simple and compound assignment. The left-hand side must be a variable or a pointer dereference.

```rs
x = 5;
x += 1;
x -= 1;
x *= 2;
x /= 2;
x %= 3;
```

### Increment and Decrement

Both prefix (`++x`, `--x`) and postfix (`x++`, `x--`) forms are supported.

```rs
x++;
++x;
x--;
--x;
```

### Pointer Operators

`&expr` takes the address of a local variable. `*expr` dereferences a pointer (peek). A dereferenced pointer may appear on the left-hand side of an assignment (poke).

```rs
let val = 42;
let ptr = &val;   // ptr holds the address of val
let n = *ptr;     // peek: read through pointer
*ptr = 99;        // poke: write through pointer
```

### Operator Precedence (high to low)

| Level | Operators                        |
|-------|----------------------------------|
| 1     | `()` `++` `--` (postfix)         |
| 2     | `-` `!` `~` `*` `&` `++` `--` (prefix) |
| 3     | `*` `/` `%`                      |
| 4     | `+` `-`                          |
| 5     | `<<` `>>`                        |
| 6     | `<` `<=` `>` `>=`               |
| 7     | `==` `!=`                        |
| 8     | `&` (bitwise)                    |
| 9     | `^`                              |
| 10    | `\|`                             |
| 11    | `&&`                             |
| 12    | `\|\|`                           |
| 13    | `=` `+=` `-=` `*=` `/=` `%=`    |

---

## Control Flow

### If / Else

The condition must be an expression. Any non-zero value is truthy. The `else` branch is optional. `else if` chains by nesting.

```rs
if (x == 1) {
    foo();
} else if (x == 2) {
    bar();
} else {
    baz();
}
```

### While

```rs
while (n > 0) {
    n -= 1;
}
```

---

## Functions

```rs
fn add(a: int, b: int) -> int {
    return a + b;
}
```

- All functions must be declared at the global scope.
- Parameters are typed. The return type follows `->` and is optional (omitting it means the function returns no value).
- `return` exits the function, optionally returning a value.
- A `return` with no expression is valid in void functions; a bare fall-off at the end of a function also returns.

### Calling a Function

```rs
let result = add(3, 4);
add(1, 2);   // return value discarded
```

---

## Calling Convention

The TeenyAT has five general-purpose registers used by the calling convention:

| Register | Saved by | Role                               |
|----------|----------|------------------------------------|
| `rA`     | Caller   | Arg 0 / return value               |
| `rB`     | Caller   | Arg 1 / scratch                    |
| `rC`     | Caller   | Arg 2 / scratch                    |
| `rD`     | Callee   | Local variable storage             |
| `rE`     | Callee   | Frame pointer                      |

### Argument Passing

Every argument is a single 16-bit word.

- **Primitive types** (`int`, `bool`, pointers): passed by value.
- The first three arguments go in `rA`, `rB`, `rC`.
- Additional arguments are pushed to the stack before the call.

### Return Values

The return value is placed in `rA`. If it fits in one word (primitives and pointers), it is returned directly.

### Stack Frame Layout

On entry to a function, the callee:
1. Pushes `rE` (saved frame pointer).
2. Sets `rE = SP` (establishes frame pointer).
3. Pushes the register arguments onto the stack as locals.

Local variables are addressed relative to `rE`. Stack-passed arguments are above `rE` (at positive offsets).

```
[rE - 0]  arg0 / first local
[rE - 1]  arg1 / second local
[rE - 2]  arg2 / third local
[rE + 1]  saved rE
[rE + 2]  return address
[rE + 3]  4th argument (if any)
[rE + 4]  5th argument (if any)
...
```

---

## Grammar (EBNF)

```ebnf
program = { stmt } ;

stmt = var_decl
     | fn_decl
     | if_stmt
     | while_stmt
     | return_stmt
     | block
     | expr_stmt ;

block = "{" { stmt } "}" ;

var_decl = ("let" | "const") IDENTIFIER [ ":" type ] "=" expression ";" ;

fn_decl = "fn" IDENTIFIER "(" [ param_list ] ")" [ "->" type ] block ;

param_list = param { "," param } ;
param      = IDENTIFIER ":" type ;

if_stmt = "if" "(" expression ")" stmt [ "else" stmt ] ;

while_stmt = "while" "(" expression ")" stmt ;

return_stmt = "return" [ expression ] ";" ;

expr_stmt = expression ";" ;

expression = assignment ;

assignment = logical_or [ ("=" | "+=" | "-=" | "*=" | "/=" | "%=") expression ] ;

logical_or  = logical_and { "||" logical_and } ;
logical_and = bit_or      { "&&" bit_or } ;
bit_or      = bit_xor     { "|"  bit_xor } ;
bit_xor     = bit_and     { "^"  bit_and } ;
bit_and     = equality    { "&"  equality } ;
equality    = comparison  { ("==" | "!=") comparison } ;
comparison  = bit_shift   { ("<" | "<=" | ">" | ">=") bit_shift } ;
bit_shift   = sum         { ("<<" | ">>") sum } ;
sum         = product     { ("+" | "-") product } ;
product     = prefix      { ("*" | "/" | "%") prefix } ;

prefix = ("-" | "!" | "~" | "*" | "&" | "++" | "--") prefix
       | postfix ;

postfix = primary { "++" | "--" | "(" [ arg_list ] ")" } ;

primary = INTEGER
        | "true"
        | "false"
        | IDENTIFIER
        | "(" expression ")" ;

arg_list = expression { "," expression } ;

type = "int"
     | "bool"
     | "*" type ;
```
