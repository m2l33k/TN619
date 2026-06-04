//! Token definitions and the bilingual keyword map.
//!
//! THE CORE BILINGUAL MECHANISM lives in `keyword()`: English and Arabic
//! spellings map to the SAME language-neutral `TokenKind`. After lexing,
//! nothing downstream knows whether the source was English or Arabic.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (language-neutral after this point)
    Let,    // let   / دع
    Var,    // var   / متغير
    Fn,     // fn    / دالة
    If,     // if    / اذا
    Else,   // else  / وإلا
    While,  // while / طالما
    For,    // for   / لكل
    In,     // in    / في
    Return, // return/ أرجع
    True,   // true  / صحيح
    False,  // false / خطأ
    Struct, // struct/ هيكل
    Enum,   // enum  / تعداد
    Match,  // match / طابق
    Impl,   // impl  / تطبيق

    // Literals
    Int(i64),
    Float(f64),
    Str(String),
    /// An interpolated string literal: a sequence of literal text and embedded
    /// expression source, e.g. `"hi {name}"`. Produced only when `{...}` is present.
    InterpStr(Vec<StrPiece>),
    Ident(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Assign,
    Bang,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,    // ,  /  ، (U+060C)
    DotDot,   // .. (exclusive range, used in `for`)
    Dot,      // .  (field access)
    Colon,    // :  (type annotations)
    PathSep,  // :: (enum variant paths)
    FatArrow, // => (match arms)
    Arrow,    // -> (function return type)
    Amp,      // &  (shared reference, as in &self)

    Eof,
}

/// One piece of an interpolated string. `Expr` holds the raw source between
/// `{` and `}`, re-parsed into an expression by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum StrPiece {
    Lit(String),
    Expr(String),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

/// The bilingual keyword table. This single function is the entire
/// "two surfaces, one semantics" guarantee for keywords.
pub fn keyword(word: &str) -> Option<TokenKind> {
    use TokenKind::*;
    let k = match word {
        "let" | "دع" => Let,
        "var" | "متغير" => Var,
        "fn" | "دالة" => Fn,
        "if" | "اذا" => If,
        "else" | "وإلا" => Else,
        "while" | "طالما" => While,
        "for" | "لكل" => For,
        "in" | "في" => In,
        "return" | "أرجع" => Return,
        "true" | "صحيح" => True,
        "false" | "خطأ" => False,
        "struct" | "هيكل" => Struct,
        "enum" | "تعداد" => Enum,
        "match" | "طابق" => Match,
        "impl" | "تطبيق" => Impl,
        _ => return None,
    };
    Some(k)
}

/// `print` / `اطبع` is a builtin function name, not a keyword — both spellings
/// resolve to the same builtin in the interpreter.
pub fn is_print_builtin(name: &str) -> bool {
    name == "print" || name == "اطبع"
}
