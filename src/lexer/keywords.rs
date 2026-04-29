use std::{collections::HashMap, sync::LazyLock};

use crate::lexer::token::TokenKind;

pub static KEYWORDS: LazyLock<HashMap<&'static str, TokenKind>> =
    LazyLock::new(|| {
        use TokenKind::*;

        let mut hm = HashMap::new();

        hm.insert("fn", Fn);
        hm.insert("let", Let);
        hm.insert("const", Const);
        hm.insert("return", Return);
        hm.insert("if", If);
        hm.insert("else", Else);
        hm.insert("while", While);
        hm.insert("for", For);
        hm.insert("repeat", Repeat);
        hm.insert("struct", Struct);
        hm.insert("true", True);
        hm.insert("false", False);
        hm.insert("int", Int);
        hm.insert("bool", Bool);

        hm
    });
