//! Token definitions and the trilingual keyword map.
//!
//! THE CORE TRILINGUAL MECHANISM lives in `keyword()`: English, Arabic, and
//! French spellings map to the SAME language-neutral `TokenKind`. After lexing,
//! nothing downstream knows which surface language the source was written in.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (language-neutral after this point)
    Let,    // let   / دع     / soit
    Var,    // var   / متغير  / variable
    Fn,     // fn    / دالة   / fonction
    If,     // if    / اذا    / si
    Else,   // else  / وإلا   / sinon
    While,  // while / طالما  / tantque
    For,    // for   / لكل    / pour
    In,     // in    / في     / dans
    Return, // return/ أرجع   / retourne
    True,   // true  / صحيح   / vrai
    False,  // false / خطأ    / faux
    Struct, // struct/ هيكل   / structure
    Enum,   // enum  / تعداد  / énum
    Match,  // match / طابق   / selon
    Impl,   // impl  / تطبيق  / implémente
    As,     // as    / كـ     / comme  (numeric cast)

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

/// The trilingual keyword table. This single function is the entire
/// "three surfaces, one semantics" guarantee for keywords.
///
/// French spellings that carry accents also accept an accent-stripped
/// fallback (`énum`/`enum`, `implémente`/`implemente`) so the language stays
/// typable on a keyboard without dead keys.
pub fn keyword(word: &str) -> Option<TokenKind> {
    use TokenKind::*;
    let k = match word {
        "let" | "دع" | "soit" => Let,
        "var" | "متغير" | "variable" => Var,
        "fn" | "دالة" | "fonction" => Fn,
        "if" | "اذا" | "si" => If,
        "else" | "وإلا" | "sinon" => Else,
        "while" | "طالما" | "tantque" => While,
        "for" | "لكل" | "pour" => For,
        "in" | "في" | "dans" => In,
        "return" | "أرجع" | "retourne" => Return,
        "true" | "صحيح" | "vrai" => True,
        "false" | "خطأ" | "faux" => False,
        "struct" | "هيكل" | "structure" => Struct,
        "enum" | "تعداد" | "énum" => Enum,
        "match" | "طابق" | "selon" => Match,
        "impl" | "تطبيق" | "implémente" | "implemente" => Impl,
        "as" | "كـ" | "comme" => As,
        _ => return None,
    };
    Some(k)
}

/// `print` / `اطبع` / `affiche` is a builtin function name, not a keyword —
/// all spellings resolve to the same builtin in the interpreter.
pub fn is_print_builtin(name: &str) -> bool {
    name == "print" || name == "اطبع" || name == "affiche"
}
