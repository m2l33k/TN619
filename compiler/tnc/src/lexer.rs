//! Hand-written bilingual lexer.
//!
//! Handles: English + Arabic identifiers/keywords, Arabic-Indic + Extended
//! (Persian) digit folding (٠-٩ / ۰-۹ -> 0-9), bilingual punctuation
//! (، -> Comma), `//` line comments.
//!
//! Performance: this lexer walks the source `&str` with a byte cursor and never
//! allocates a `Vec<char>`. Keywords are matched against a borrowed `&str` slice
//! (zero allocation), integer literals are accumulated directly into an `i64`,
//! and heap allocation happens ONLY for `Ident` and `Str` token payloads.
//!
//! Deferred (require external crates): Unicode NFC normalization and
//! Trojan-Source / bidi-control rejection — documented hooks, see Phase 3 roadmap.

use crate::token::{keyword, Token, TokenKind};

pub struct Lexer<'a> {
    src: &'a str,
    pos: usize, // byte offset into `src`
    line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src, pos: 0, line: 1 }
    }

    /// The next char without consuming it. O(1): decodes a single scalar.
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }
    /// The char after `peek`, without consuming.
    fn peek2(&self) -> Option<char> {
        let mut it = self.src[self.pos..].chars();
        it.next();
        it.next()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        // SECURITY (Trojan Source / CVE-2021-42574): reject bidirectional text
        // control characters anywhere in the source. These are invisible and can
        // make code *display* differently from how it *executes*. They are never
        // needed in legitimate source (the Unicode bidi algorithm renders Arabic
        // correctly without them), so we reject rather than lint. Same codepoint
        // set rustc uses (rustc_ast::util::unicode::TEXT_FLOW_CONTROL_CHARS).
        self.reject_bidi_control_chars()?;

        // Heuristic capacity: most tokens are a few bytes long.
        let mut out = Vec::with_capacity(self.src.len() / 4 + 8);
        loop {
            self.skip_trivia();
            let line = self.line;
            let c = match self.peek() {
                None => {
                    out.push(Token { kind: TokenKind::Eof, line });
                    return Ok(out);
                }
                Some(c) => c,
            };

            let kind = if is_ident_start(c) {
                self.lex_word()
            } else if is_digit(c) {
                self.lex_number()?
            } else if c == '"' {
                self.lex_string()?
            } else {
                self.lex_symbol()?
            };
            out.push(Token { kind, line });
        }
    }

    /// One O(n) pre-scan rejecting bidi control characters (Trojan Source),
    /// reporting the first offender with its line and codepoint.
    fn reject_bidi_control_chars(&self) -> Result<(), String> {
        let mut line = 1usize;
        for c in self.src.chars() {
            if c == '\n' {
                line += 1;
            } else if is_bidi_control(c) {
                return Err(format!(
                    "line {}: disallowed bidirectional control character U+{:04X} \
                     (Trojan Source defense — invisible reordering characters are not allowed)",
                    line, c as u32
                ));
            }
        }
        Ok(())
    }

    /// Whitespace (incl. newlines), `;` (optional terminators), and `//` comments.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() || c == ';' => {
                    self.bump();
                }
                Some('/') if self.peek2() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    /// Identifier or keyword. The scanned text is matched against the keyword
    /// table as a borrowed slice — no allocation for keywords; only `Ident`
    /// payloads are copied onto the heap.
    fn lex_word(&mut self) -> TokenKind {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                self.bump();
            } else {
                break;
            }
        }
        let word = &self.src[start..self.pos];
        keyword(word).unwrap_or_else(|| TokenKind::Ident(word.to_string()))
    }

    /// Integer literal. Accumulates the value directly (no temporary string),
    /// folding ASCII, Arabic-Indic, and Persian digits to one `i64`.
    fn lex_number(&mut self) -> Result<TokenKind, String> {
        let mut val: i64 = 0;
        let start_line = self.line;
        while let Some(c) = self.peek() {
            match digit_value(c) {
                Some(d) => {
                    val = val
                        .checked_mul(10)
                        .and_then(|v| v.checked_add(d as i64))
                        .ok_or_else(|| {
                            format!("line {}: integer literal is too large", start_line)
                        })?;
                    self.bump();
                }
                None => break,
            }
        }
        Ok(TokenKind::Int(val))
    }

    fn lex_string(&mut self) -> Result<TokenKind, String> {
        self.bump(); // opening "
        let mut s = String::new();
        loop {
            match self.bump() {
                None => return Err(format!("line {}: unterminated string literal", self.line)),
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(o) => s.push(o),
                    None => return Err(format!("line {}: unterminated escape", self.line)),
                },
                Some(c) => s.push(c),
            }
        }
        Ok(TokenKind::Str(s))
    }

    fn lex_symbol(&mut self) -> Result<TokenKind, String> {
        let line = self.line;
        let c = self.bump().unwrap();
        use TokenKind::*;
        let kind = match c {
            '+' => Plus,
            '-' => {
                if self.peek() == Some('>') {
                    self.bump();
                    Arrow
                } else {
                    Minus
                }
            }
            '*' => Star,
            '/' => Slash,
            '%' => Percent,
            '(' => LParen,
            ')' => RParen,
            '{' => LBrace,
            '}' => RBrace,
            ',' | '،' => Comma, // bilingual comma fold
            '&' => Amp,
            '=' => {
                if self.peek() == Some('=') {
                    self.bump();
                    EqEq
                } else if self.peek() == Some('>') {
                    self.bump();
                    FatArrow
                } else {
                    Assign
                }
            }
            ':' => {
                if self.peek() == Some(':') {
                    self.bump();
                    PathSep
                } else {
                    Colon
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Ne
                } else {
                    Bang
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Le
                } else {
                    Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.bump();
                    Ge
                } else {
                    Gt
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.bump();
                    DotDot
                } else {
                    Dot
                }
            }
            other => return Err(format!("line {}: unexpected character '{}'", line, other)),
        };
        Ok(kind)
    }
}

/// An identifier starts with a letter or `_` (never a digit — a leading digit
/// begins a number literal).
fn is_ident_start(c: char) -> bool {
    c == '_' || (c.is_alphabetic() && digit_value(c).is_none())
}
/// ...and may continue with letters, digits (incl. Arabic-Indic), or `_`.
/// So `count1`, `د1`, and `عداد٢` are all valid identifiers.
fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

fn is_digit(c: char) -> bool {
    digit_value(c).is_some()
}

/// Bidirectional text-flow control characters (the Trojan Source set). Same list
/// rustc rejects: LRE/RLE/PDF/LRO/RLO embeddings + LRI/RLI/FSI/PDI isolates.
fn is_bidi_control(c: char) -> bool {
    matches!(
        c,
        '\u{202A}' | '\u{202B}' | '\u{202C}' | '\u{202D}' | '\u{202E}'
            | '\u{2066}' | '\u{2067}' | '\u{2068}' | '\u{2069}'
    )
}

/// Numeric value of a digit char. Folds ASCII `0-9`, Arabic-Indic `٠-٩`
/// (U+0660..U+0669), and Extended Arabic-Indic / Persian `۰-۹`
/// (U+06F0..U+06F9) to a single numeric value — the digit-folding mechanism.
fn digit_value(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        '\u{0660}'..='\u{0669}' => Some(c as u32 - 0x0660),
        '\u{06F0}'..='\u{06F9}' => Some(c as u32 - 0x06F0),
        _ => None,
    }
}
