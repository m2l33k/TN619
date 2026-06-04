//! Recursive-descent parser (statements/items) + Pratt parser (expressions).
//!
//! Statement boundaries are structural for the MVP: expression parsing stops at
//! any non-operator token, so newlines need no explicit terminator token.
//! (Proper Go-style automatic-semicolon-insertion comes in a later iteration.)

use crate::ast::*;
use crate::token::{Token, TokenKind};

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
    /// Whether `Ident {` should be read as a struct literal. Disabled while
    /// parsing if/while/for conditions and match scrutinees, where `{` opens a
    /// block (same disambiguation Rust uses).
    allow_struct_lit: bool,
}

type PResult<T> = Result<T, String>;

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Parser {
            toks,
            pos: 0,
            allow_struct_lit: true,
        }
    }

    /// Run `f` with struct-literal parsing disabled (for conditions/scrutinees).
    fn no_struct<T>(&mut self, f: impl FnOnce(&mut Self) -> PResult<T>) -> PResult<T> {
        let saved = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let r = f(self);
        self.allow_struct_lit = saved;
        r
    }
    /// Run `f` with struct-literal parsing enabled (inside brackets/arg lists).
    fn with_struct<T>(&mut self, f: impl FnOnce(&mut Self) -> PResult<T>) -> PResult<T> {
        let saved = self.allow_struct_lit;
        self.allow_struct_lit = true;
        let r = f(self);
        self.allow_struct_lit = saved;
        r
    }

    fn peek(&self) -> &TokenKind {
        &self.toks[self.pos].kind
    }
    fn line(&self) -> usize {
        self.toks[self.pos].line
    }
    fn advance(&mut self) -> TokenKind {
        let k = self.toks[self.pos].kind.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        k
    }
    fn eat(&mut self, k: &TokenKind) -> bool {
        if self.peek() == k {
            self.advance();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, k: TokenKind) -> PResult<()> {
        if self.peek() == &k {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "line {}: expected {:?}, found {:?}",
                self.line(),
                k,
                self.peek()
            ))
        }
    }

    pub fn parse_program(&mut self) -> PResult<Program> {
        let mut items = Vec::new();
        while self.peek() != &TokenKind::Eof {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> PResult<Item> {
        match self.peek() {
            TokenKind::Fn => Ok(Item::Fn(self.parse_fn()?)),
            TokenKind::Struct => Ok(Item::Struct(self.parse_struct()?)),
            TokenKind::Enum => Ok(Item::Enum(self.parse_enum()?)),
            TokenKind::Impl => Ok(Item::Impl(self.parse_impl()?)),
            other => Err(format!(
                "line {}: expected fn / struct / enum / impl, found {:?}",
                self.line(),
                other
            )),
        }
    }

    fn parse_struct(&mut self) -> PResult<StructDecl> {
        self.expect(TokenKind::Struct)?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            let fname = self.parse_ident()?;
            self.expect(TokenKind::Colon)?;
            let fty = self.parse_type()?;
            fields.push((fname, fty));
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(StructDecl { name, fields })
    }

    fn parse_enum(&mut self) -> PResult<EnumDecl> {
        self.expect(TokenKind::Enum)?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            let vname = self.parse_ident()?;
            let mut payloads = Vec::new();
            if self.eat(&TokenKind::LParen) {
                if self.peek() != &TokenKind::RParen {
                    loop {
                        payloads.push(self.parse_type()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RParen)?;
            }
            variants.push(VariantDecl {
                name: vname,
                payloads,
            });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(EnumDecl { name, variants })
    }

    fn parse_fn(&mut self) -> PResult<FnDecl> {
        self.expect(TokenKind::Fn)?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if self.peek() != &TokenKind::RParen {
            loop {
                let pname = self.parse_ident()?;
                self.expect(TokenKind::Colon)?;
                let ty = self.parse_type()?;
                params.push(Param { name: pname, ty });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;
        let ret = if self.eat(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(FnDecl {
            name,
            params,
            ret,
            body,
        })
    }

    /// A type is a single identifier for the MVP (primitive or user-defined name).
    fn parse_type(&mut self) -> PResult<String> {
        self.parse_ident()
    }

    fn parse_one_param(&mut self) -> PResult<Param> {
        let name = self.parse_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(Param { name, ty })
    }

    /// Consumes a `self` / `الذات` identifier if present.
    fn eat_self(&mut self) -> bool {
        if let TokenKind::Ident(s) = self.peek() {
            if s == "self" || s == "الذات" {
                self.advance();
                return true;
            }
        }
        false
    }

    fn parse_impl(&mut self) -> PResult<ImplBlock> {
        self.expect(TokenKind::Impl)?;
        let type_name = self.parse_ident()?;
        self.expect(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            methods.push(self.parse_method()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ImplBlock { type_name, methods })
    }

    fn parse_method(&mut self) -> PResult<MethodDecl> {
        self.expect(TokenKind::Fn)?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LParen)?;

        let mut self_kind = SelfKind::None;
        let mut params = Vec::new();
        if self.peek() == &TokenKind::Amp {
            self.advance();
            if !self.eat_self() {
                return Err(format!("line {}: expected `self` after `&`", self.line()));
            }
            self_kind = SelfKind::Ref;
            while self.eat(&TokenKind::Comma) {
                params.push(self.parse_one_param()?);
            }
        } else if self.eat_self() {
            self_kind = SelfKind::Value;
            while self.eat(&TokenKind::Comma) {
                params.push(self.parse_one_param()?);
            }
        } else if self.peek() != &TokenKind::RParen {
            params.push(self.parse_one_param()?);
            while self.eat(&TokenKind::Comma) {
                params.push(self.parse_one_param()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        let ret = if self.eat(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(MethodDecl {
            self_kind,
            func: FnDecl {
                name,
                params,
                ret,
                body,
            },
        })
    }

    fn parse_ident(&mut self) -> PResult<String> {
        match self.advance() {
            TokenKind::Ident(s) => Ok(s),
            other => Err(format!(
                "line {}: expected identifier, found {:?}",
                self.line(),
                other
            )),
        }
    }

    fn parse_block(&mut self) -> PResult<Block> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.peek() {
            TokenKind::Let | TokenKind::Var => self.parse_let(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Ident(_) if self.toks[self.pos + 1].kind == TokenKind::Assign => {
                let name = self.parse_ident()?;
                self.expect(TokenKind::Assign)?;
                let value = self.parse_expr(0)?;
                Ok(Stmt::Assign { name, value })
            }
            _ => Ok(Stmt::Expr(self.parse_expr(0)?)),
        }
    }

    fn parse_let(&mut self) -> PResult<Stmt> {
        let mutable = self.advance() == TokenKind::Var;
        let name = self.parse_ident()?;
        let ty_ann = if self.eat(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Assign)?;
        let init = self.parse_expr(0)?;
        Ok(Stmt::Let {
            name,
            mutable,
            ty_ann,
            init,
        })
    }

    fn parse_while(&mut self) -> PResult<Stmt> {
        self.expect(TokenKind::While)?;
        let cond = self.no_struct(|p| p.parse_expr(0))?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_for(&mut self) -> PResult<Stmt> {
        self.expect(TokenKind::For)?;
        let var = self.parse_ident()?;
        self.expect(TokenKind::In)?;
        let start = self.no_struct(|p| p.parse_expr(0))?;
        self.expect(TokenKind::DotDot)?;
        let end = self.no_struct(|p| p.parse_expr(0))?;
        let body = self.parse_block()?;
        Ok(Stmt::For {
            var,
            start,
            end,
            body,
        })
    }

    fn parse_return(&mut self) -> PResult<Stmt> {
        self.expect(TokenKind::Return)?;
        // A return may be bare (followed by `}`) or carry an expression.
        if self.peek() == &TokenKind::RBrace {
            Ok(Stmt::Return(None))
        } else {
            Ok(Stmt::Return(Some(self.parse_expr(0)?)))
        }
    }

    // ---- Pratt expression parser ----

    fn parse_expr(&mut self, min_bp: u8) -> PResult<Expr> {
        let mut lhs = self.parse_postfix()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::Ne => BinOp::Ne,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            let (l_bp, r_bp) = infix_bp(op);
            if l_bp < min_bp {
                break;
            }
            self.advance();
            let rhs = self.parse_expr(r_bp)?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    /// Wraps a prefix expression with trailing `.field` accesses and
    /// `.method(args)` calls.
    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut e = self.parse_prefix()?;
        while self.peek() == &TokenKind::Dot {
            self.advance();
            let name = self.parse_ident()?;
            if self.peek() == &TokenKind::LParen {
                let args = self.parse_call_args()?;
                e = Expr::MethodCall {
                    receiver: Box::new(e),
                    method: name,
                    args,
                };
            } else {
                e = Expr::Field {
                    base: Box::new(e),
                    field: name,
                };
            }
        }
        Ok(e)
    }

    fn parse_prefix(&mut self) -> PResult<Expr> {
        match self.peek().clone() {
            TokenKind::Minus => {
                self.advance();
                let rhs = self.parse_expr(7)?;
                Ok(Expr::Unary {
                    op: UnOp::Neg,
                    rhs: Box::new(rhs),
                })
            }
            TokenKind::Bang => {
                self.advance();
                let rhs = self.parse_expr(7)?;
                Ok(Expr::Unary {
                    op: UnOp::Not,
                    rhs: Box::new(rhs),
                })
            }
            TokenKind::Int(n) => {
                self.advance();
                Ok(Expr::Int(n))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Str(s))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.with_struct(|p| p.parse_expr(0))?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::If => self.parse_if(),
            TokenKind::Match => self.parse_match(),
            TokenKind::Ident(name) => {
                self.advance();
                if self.peek() == &TokenKind::PathSep {
                    // Path: Type::member  or  Type::member(args)
                    // (enum variant OR associated function — resolved later)
                    self.advance();
                    let member = self.parse_ident()?;
                    let args = if self.peek() == &TokenKind::LParen {
                        self.parse_call_args()?
                    } else {
                        Vec::new()
                    };
                    Ok(Expr::Path {
                        ty: name,
                        member,
                        args,
                    })
                } else if self.peek() == &TokenKind::LParen {
                    // Function call OR bare tuple-variant construction (resolved at runtime).
                    let args = self.parse_call_args()?;
                    Ok(Expr::Call { callee: name, args })
                } else if self.allow_struct_lit && self.peek() == &TokenKind::LBrace {
                    self.parse_struct_lit(name)
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            other => Err(format!(
                "line {}: unexpected token in expression: {:?}",
                self.line(),
                other
            )),
        }
    }

    fn parse_if(&mut self) -> PResult<Expr> {
        self.expect(TokenKind::If)?;
        let cond = self.no_struct(|p| p.parse_expr(0))?;
        let then_b = self.parse_block()?;
        let els = if self.eat(&TokenKind::Else) {
            if self.peek() == &TokenKind::If {
                Some(Box::new(self.parse_if()?))
            } else {
                Some(Box::new(Expr::Block(self.parse_block()?)))
            }
        } else {
            None
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then_b,
            els,
        })
    }

    fn parse_call_args(&mut self) -> PResult<Vec<Expr>> {
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        if self.peek() != &TokenKind::RParen {
            // Inside an arg list, struct literals are unambiguous again.
            self.with_struct(|p| {
                loop {
                    args.push(p.parse_expr(0)?);
                    if !p.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                Ok(())
            })?;
        }
        self.expect(TokenKind::RParen)?;
        Ok(args)
    }

    fn parse_struct_lit(&mut self, name: String) -> PResult<Expr> {
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        self.with_struct(|p| {
            while p.peek() != &TokenKind::RBrace && p.peek() != &TokenKind::Eof {
                let fname = p.parse_ident()?;
                let value = if p.eat(&TokenKind::Colon) {
                    p.parse_expr(0)?
                } else {
                    // Field shorthand: `User { name }` == `User { name: name }`.
                    Expr::Ident(fname.clone())
                };
                fields.push((fname, value));
                if !p.eat(&TokenKind::Comma) {
                    break;
                }
            }
            Ok(())
        })?;
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::StructLit { name, fields })
    }

    fn parse_match(&mut self) -> PResult<Expr> {
        self.expect(TokenKind::Match)?;
        let scrutinee = self.no_struct(|p| p.parse_expr(0))?;
        self.expect(TokenKind::LBrace)?;
        let mut arms = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::FatArrow)?;
            let body = self.with_struct(|p| p.parse_expr(0))?;
            arms.push(MatchArm { pattern, body });
            self.eat(&TokenKind::Comma); // optional trailing comma
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }

    fn parse_pattern(&mut self) -> PResult<Pattern> {
        match self.peek().clone() {
            TokenKind::Int(n) => {
                self.advance();
                Ok(Pattern::Int(n))
            }
            TokenKind::Minus => {
                // negative integer literal pattern
                self.advance();
                match self.advance() {
                    TokenKind::Int(n) => Ok(Pattern::Int(-n)),
                    other => Err(format!(
                        "line {}: expected number after '-', found {:?}",
                        self.line(),
                        other
                    )),
                }
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Pattern::Str(s))
            }
            TokenKind::True => {
                self.advance();
                Ok(Pattern::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Pattern::Bool(false))
            }
            TokenKind::Ident(name) => {
                self.advance();
                if name == "_" {
                    return Ok(Pattern::Wildcard);
                }
                if self.peek() == &TokenKind::PathSep {
                    self.advance();
                    let variant = self.parse_ident()?;
                    let subs = self.parse_pattern_subs()?;
                    Ok(Pattern::Variant {
                        enum_name: Some(name),
                        name: variant,
                        subs,
                    })
                } else if self.peek() == &TokenKind::LParen {
                    let subs = self.parse_pattern_subs()?;
                    Ok(Pattern::Variant {
                        enum_name: None,
                        name,
                        subs,
                    })
                } else {
                    // Bare name: binding or unit variant (resolved at match time).
                    Ok(Pattern::Ident(name))
                }
            }
            other => Err(format!(
                "line {}: invalid pattern, found {:?}",
                self.line(),
                other
            )),
        }
    }

    fn parse_pattern_subs(&mut self) -> PResult<Vec<Pattern>> {
        let mut subs = Vec::new();
        if self.eat(&TokenKind::LParen) {
            if self.peek() != &TokenKind::RParen {
                loop {
                    subs.push(self.parse_pattern()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RParen)?;
        }
        Ok(subs)
    }
}

fn infix_bp(op: BinOp) -> (u8, u8) {
    use BinOp::*;
    match op {
        Eq | Ne | Lt | Le | Gt | Ge => (3, 4),
        Add | Sub => (5, 6),
        Mul | Div | Rem => (7, 8),
    }
}
