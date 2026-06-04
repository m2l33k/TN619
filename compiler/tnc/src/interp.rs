//! Tree-walking interpreter — the MVP backend.
//!
//! This stands in for the eventual LLVM/Cranelift codegen. It consumes the same
//! language-neutral AST, so swapping in a real backend later does not touch the
//! lexer/parser/AST front-end.

use crate::ast::*;
use crate::token::is_print_builtin;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Unit,
    Struct {
        name: String,
        fields: Vec<(String, Value)>,
    },
    Enum {
        enum_name: String,
        variant: String,
        data: Vec<Value>,
    },
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Str(s) => write!(f, "{}", s),
            Value::Unit => write!(f, "()"),
            Value::Struct { name, fields } => {
                write!(f, "{} {{ ", name)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            Value::Enum { variant, data, .. } => {
                write!(f, "{}", variant)?;
                if !data.is_empty() {
                    write!(f, "(")?;
                    for (i, v) in data.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
        }
    }
}

/// Non-local control flow during evaluation.
enum Flow {
    Return(Value),
    Error(String),
}
type EResult<T> = Result<T, Flow>;

type Scope = HashMap<String, (Value, bool)>; // name -> (value, mutable)

/// The type name a value belongs to, for method dispatch.
fn type_name_of(v: &Value) -> Option<String> {
    match v {
        Value::Struct { name, .. } => Some(name.clone()),
        Value::Enum { enum_name, .. } => Some(enum_name.clone()),
        _ => None,
    }
}

pub struct Interp {
    funcs: HashMap<String, FnDecl>,
    structs: HashMap<String, StructDecl>,
    /// variant name -> (enum name, arity). Lets bare `Circle(..)` / `Red`
    /// resolve to enum construction without an explicit `Enum::` prefix.
    variants: HashMap<String, (String, usize)>,
    /// (type name, method name) -> method.
    methods: HashMap<(String, String), MethodDecl>,
    scopes: Vec<Scope>, // current call frame's scope stack
}

impl Interp {
    pub fn new() -> Self {
        Interp {
            funcs: HashMap::new(),
            structs: HashMap::new(),
            variants: HashMap::new(),
            methods: HashMap::new(),
            scopes: vec![],
        }
    }

    pub fn run(&mut self, prog: &Program) -> Result<(), String> {
        for item in &prog.items {
            match item {
                Item::Fn(f) => {
                    self.funcs.insert(f.name.clone(), f.clone());
                }
                Item::Struct(s) => {
                    self.structs.insert(s.name.clone(), s.clone());
                }
                Item::Enum(e) => {
                    for v in &e.variants {
                        if let Some((other, _)) = self.variants.get(&v.name) {
                            return Err(format!(
                                "variant `{}` is declared in both `{}` and `{}`",
                                v.name, other, e.name
                            ));
                        }
                        self.variants
                            .insert(v.name.clone(), (e.name.clone(), v.payloads.len()));
                    }
                }
                Item::Impl(b) => {
                    for m in &b.methods {
                        self.methods
                            .insert((b.type_name.clone(), m.func.name.clone()), m.clone());
                    }
                }
            }
        }
        // Entry point: main / رئيسي
        let main = self
            .funcs
            .get("main")
            .or_else(|| self.funcs.get("رئيسي"))
            .cloned()
            .ok_or_else(|| "no entry point: define `fn main()` or `دالة رئيسي()`".to_string())?;
        match self.call(&main, vec![]) {
            Ok(_) => Ok(()),
            Err(Flow::Error(e)) => Err(e),
            Err(Flow::Return(_)) => Ok(()),
        }
    }

    fn call(&mut self, f: &FnDecl, args: Vec<Value>) -> EResult<Value> {
        self.call_with_self(f, None, args)
    }

    fn call_with_self(
        &mut self,
        f: &FnDecl,
        self_val: Option<Value>,
        args: Vec<Value>,
    ) -> EResult<Value> {
        if args.len() != f.params.len() {
            return Err(Flow::Error(format!(
                "function `{}` expects {} args, got {}",
                f.name,
                f.params.len(),
                args.len()
            )));
        }
        // Functions do NOT see the caller's locals: install a fresh scope stack.
        let saved = std::mem::take(&mut self.scopes);
        self.scopes.push(Scope::new());
        if let Some(sv) = self_val {
            // Bound under both spellings so EN/AR method bodies both work.
            self.scopes
                .last_mut()
                .unwrap()
                .insert("self".into(), (sv.clone(), false));
            self.scopes
                .last_mut()
                .unwrap()
                .insert("الذات".into(), (sv, false));
        }
        for (p, a) in f.params.iter().zip(args) {
            self.scopes
                .last_mut()
                .unwrap()
                .insert(p.name.clone(), (a, false));
        }
        let result = match self.eval_block(&f.body) {
            Ok(v) => Ok(v),
            Err(Flow::Return(v)) => Ok(v),
            Err(e) => Err(e),
        };
        self.scopes = saved;
        result
    }

    fn eval_block(&mut self, b: &Block) -> EResult<Value> {
        self.scopes.push(Scope::new());
        let mut last = Value::Unit;
        for s in &b.stmts {
            last = match self.eval_stmt(s) {
                Ok(v) => v,
                Err(e) => {
                    self.scopes.pop();
                    return Err(e);
                }
            };
        }
        self.scopes.pop();
        Ok(last)
    }

    fn eval_stmt(&mut self, s: &Stmt) -> EResult<Value> {
        match s {
            Stmt::Let {
                name,
                mutable,
                init,
                ..
            } => {
                let v = self.eval_expr(init)?;
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(name.clone(), (v, *mutable));
                Ok(Value::Unit)
            }
            Stmt::Assign { name, value } => {
                let v = self.eval_expr(value)?;
                self.assign(name, v)?;
                Ok(Value::Unit)
            }
            Stmt::While { cond, body } => {
                while self.truthy(cond)? {
                    self.eval_block(body)?;
                }
                Ok(Value::Unit)
            }
            Stmt::For {
                var,
                start,
                end,
                body,
            } => {
                let s = self.as_int(start)?;
                let e = self.as_int(end)?;
                let mut i = s;
                while i < e {
                    self.scopes.push(Scope::new());
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(var.clone(), (Value::Int(i), false));
                    let r = self.eval_block(body);
                    self.scopes.pop();
                    r?;
                    i += 1;
                }
                Ok(Value::Unit)
            }
            Stmt::Return(opt) => {
                let v = match opt {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Unit,
                };
                Err(Flow::Return(v))
            }
            Stmt::Expr(e) => self.eval_expr(e),
        }
    }

    /// Evaluate `e`, tagging any runtime error with its source line.
    fn eval_expr(&mut self, e: &Expr) -> EResult<Value> {
        self.eval_expr_kind(e).map_err(|f| match f {
            Flow::Error(m) if !m.starts_with("line ") => {
                Flow::Error(format!("line {}: {}", e.line, m))
            }
            other => other,
        })
    }

    fn eval_expr_kind(&mut self, e: &Expr) -> EResult<Value> {
        match &e.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Str(s) => Ok(Value::Str(s.clone())),
            ExprKind::StrInterp(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        StrPart::Lit(s) => out.push_str(s),
                        StrPart::Expr(e) => out.push_str(&self.eval_expr(e)?.to_string()),
                    }
                }
                Ok(Value::Str(out))
            }
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Ident(name) => {
                if let Some(v) = self.lookup(name) {
                    Ok(v)
                } else if let Some((enum_name, 0)) = self.variants.get(name).cloned() {
                    // Bare unit variant, e.g. `None` / `Red`.
                    Ok(Value::Enum {
                        enum_name,
                        variant: name.clone(),
                        data: vec![],
                    })
                } else {
                    Err(Flow::Error(format!("cannot find `{}` in scope", name)))
                }
            }
            ExprKind::Unary { op, rhs } => {
                let v = self.eval_expr(rhs)?;
                match (op, v) {
                    (UnOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
                    (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (op, v) => Err(Flow::Error(format!("invalid unary {:?} on {}", op, v))),
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.eval_expr(lhs)?;
                let r = self.eval_expr(rhs)?;
                self.eval_binary(*op, l, r)
            }
            ExprKind::Call { callee, args } => {
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.eval_expr(a)?);
                }
                if is_print_builtin(callee) {
                    let line: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                    println!("{}", line.join(" "));
                    return Ok(Value::Unit);
                }
                // Bare tuple-variant construction, e.g. `Some(x)` / `Circle(r)`.
                if let Some((enum_name, arity)) = self.variants.get(callee).cloned() {
                    if vals.len() != arity {
                        return Err(Flow::Error(format!(
                            "variant `{}` expects {} value(s), got {}",
                            callee,
                            arity,
                            vals.len()
                        )));
                    }
                    return Ok(Value::Enum {
                        enum_name,
                        variant: callee.clone(),
                        data: vals,
                    });
                }
                let f = self
                    .funcs
                    .get(callee)
                    .cloned()
                    .ok_or_else(|| Flow::Error(format!("cannot find function `{}`", callee)))?;
                self.call(&f, vals)
            }
            ExprKind::If { cond, then_b, els } => {
                if self.truthy(cond)? {
                    self.eval_block(then_b)
                } else if let Some(e) = els {
                    self.eval_expr(e)
                } else {
                    Ok(Value::Unit)
                }
            }
            ExprKind::Block(b) => self.eval_block(b),
            ExprKind::StructLit { name, fields } => {
                let decl = self
                    .structs
                    .get(name)
                    .cloned()
                    .ok_or_else(|| Flow::Error(format!("unknown struct `{}`", name)))?;
                // Every declared field must be supplied exactly once.
                for (fname, _) in fields {
                    if !decl.fields.iter().any(|(f, _)| f == fname) {
                        return Err(Flow::Error(format!(
                            "struct `{}` has no field `{}`",
                            name, fname
                        )));
                    }
                }
                if fields.len() != decl.fields.len() {
                    return Err(Flow::Error(format!(
                        "struct `{}` expects {} field(s), got {}",
                        name,
                        decl.fields.len(),
                        fields.len()
                    )));
                }
                let mut out = Vec::new();
                for (fname, fexpr) in fields {
                    let v = self.eval_expr(fexpr)?;
                    out.push((fname.clone(), v));
                }
                Ok(Value::Struct {
                    name: name.clone(),
                    fields: out,
                })
            }
            ExprKind::Field { base, field } => {
                let b = self.eval_expr(base)?;
                match b {
                    Value::Struct { name, fields } => fields
                        .iter()
                        .find(|(k, _)| k == field)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| {
                            Flow::Error(format!("struct `{}` has no field `{}`", name, field))
                        }),
                    other => Err(Flow::Error(format!(
                        "cannot access field `{}` on {}",
                        field, other
                    ))),
                }
            }
            ExprKind::Path { ty, member, args } => {
                // Enum construction takes priority: `Shape::Circle(..)` / `Color::Red`.
                if let Some((en, _)) = self.variants.get(member).cloned() {
                    if &en == ty {
                        let mut data = Vec::new();
                        for a in args {
                            data.push(self.eval_expr(a)?);
                        }
                        return Ok(Value::Enum {
                            enum_name: ty.clone(),
                            variant: member.clone(),
                            data,
                        });
                    }
                }
                // Otherwise an associated function: `Point::new(..)`.
                if let Some(m) = self.methods.get(&(ty.clone(), member.clone())).cloned() {
                    if m.self_kind != SelfKind::None {
                        return Err(Flow::Error(format!(
                            "`{}::{}` is a method; call it on a value with `.`",
                            ty, member
                        )));
                    }
                    let mut vals = Vec::new();
                    for a in args {
                        vals.push(self.eval_expr(a)?);
                    }
                    return self.call_with_self(&m.func, None, vals);
                }
                Err(Flow::Error(format!("no `{}::{}`", ty, member)))
            }
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                let recv = self.eval_expr(receiver)?;
                let tyname = type_name_of(&recv)
                    .ok_or_else(|| Flow::Error(format!("type `{}` has no methods", recv)))?;
                let m = self
                    .methods
                    .get(&(tyname.clone(), method.clone()))
                    .cloned()
                    .ok_or_else(|| {
                        Flow::Error(format!("`{}` has no method `{}`", tyname, method))
                    })?;
                if m.self_kind == SelfKind::None {
                    return Err(Flow::Error(format!(
                        "`{}::{}` is an associated function; call it as `{}::{}(..)`",
                        tyname, method, tyname, method
                    )));
                }
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.eval_expr(a)?);
                }
                self.call_with_self(&m.func, Some(recv), vals)
            }
            ExprKind::Match { scrutinee, arms } => {
                let val = self.eval_expr(scrutinee)?;
                for arm in arms {
                    self.scopes.push(Scope::new());
                    if self.match_pattern(&arm.pattern, &val) {
                        let r = self.eval_expr(&arm.body);
                        self.scopes.pop();
                        return r;
                    }
                    self.scopes.pop();
                }
                // Compile-time exhaustiveness checking is a type-checker feature
                // (deferred); for now an unmatched value is a runtime error.
                Err(Flow::Error(format!("no match arm matched value: {}", val)))
            }
        }
    }

    /// Tries to match `val` against `pat`, inserting any bindings into the
    /// current (top) scope. The caller discards that scope on failure, so
    /// partial bindings need no rollback.
    fn match_pattern(&mut self, pat: &Pattern, val: &Value) -> bool {
        match pat {
            Pattern::Wildcard => true,
            Pattern::Int(n) => matches!(val, Value::Int(m) if m == n),
            Pattern::Bool(b) => matches!(val, Value::Bool(m) if m == b),
            Pattern::Str(s) => matches!(val, Value::Str(m) if m == s),
            Pattern::Ident(name) => {
                // Bare name: a known unit variant matches that variant;
                // otherwise it binds the value.
                if let Some((_, 0)) = self.variants.get(name) {
                    matches!(val, Value::Enum { variant, data, .. }
                        if variant == name && data.is_empty())
                } else {
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), (val.clone(), false));
                    true
                }
            }
            Pattern::Variant {
                enum_name,
                name,
                subs,
            } => match val {
                Value::Enum {
                    enum_name: en,
                    variant,
                    data,
                } => {
                    if variant != name {
                        return false;
                    }
                    if let Some(expected) = enum_name {
                        if expected != en {
                            return false;
                        }
                    }
                    if data.len() != subs.len() {
                        return false;
                    }
                    // Clone payloads to avoid borrowing self while matching subs.
                    let data = data.clone();
                    subs.iter()
                        .zip(data.iter())
                        .all(|(p, v)| self.match_pattern(p, v))
                }
                _ => false,
            },
        }
    }

    fn eval_binary(&self, op: BinOp, l: Value, r: Value) -> EResult<Value> {
        use BinOp::*;
        match op {
            Add | Sub | Mul | Div | Rem => {
                let (a, b) = match (l, r) {
                    (Value::Int(a), Value::Int(b)) => (a, b),
                    (l, r) => {
                        return Err(Flow::Error(format!(
                            "arithmetic on non-ints: {} {:?} {}",
                            l, op, r
                        )))
                    }
                };
                let v = match op {
                    Add => a.checked_add(b),
                    Sub => a.checked_sub(b),
                    Mul => a.checked_mul(b),
                    Div => {
                        if b == 0 {
                            return Err(Flow::Error("division by zero".into()));
                        }
                        Some(a / b)
                    }
                    Rem => {
                        if b == 0 {
                            return Err(Flow::Error("remainder by zero".into()));
                        }
                        Some(a % b)
                    }
                    _ => unreachable!(),
                };
                // Secure-by-default: overflow is an error, never silent wraparound.
                v.map(Value::Int)
                    .ok_or_else(|| Flow::Error("integer overflow".into()))
            }
            Eq | Ne | Lt | Le | Gt | Ge => {
                let ord = self.compare(&l, &r)?;
                let b = match op {
                    Eq => ord == 0,
                    Ne => ord != 0,
                    Lt => ord < 0,
                    Le => ord <= 0,
                    Gt => ord > 0,
                    Ge => ord >= 0,
                    _ => unreachable!(),
                };
                Ok(Value::Bool(b))
            }
        }
    }

    fn compare(&self, l: &Value, r: &Value) -> EResult<i32> {
        let c = match (l, r) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Str(a), Value::Str(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (l, r) => return Err(Flow::Error(format!("cannot compare {} and {}", l, r))),
        };
        Ok(match c {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        })
    }

    fn truthy(&mut self, e: &Expr) -> EResult<bool> {
        match self.eval_expr(e)? {
            Value::Bool(b) => Ok(b),
            v => Err(Flow::Error(format!(
                "condition must be a bool, found {}",
                v
            ))),
        }
    }

    fn as_int(&mut self, e: &Expr) -> EResult<i64> {
        match self.eval_expr(e)? {
            Value::Int(n) => Ok(n),
            v => Err(Flow::Error(format!("expected an integer, found {}", v))),
        }
    }

    fn lookup(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some((v, _)) = scope.get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    fn assign(&mut self, name: &str, v: Value) -> EResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some((slot, mutable)) = scope.get_mut(name) {
                if !*mutable {
                    return Err(Flow::Error(format!(
                        "cannot assign to immutable binding `{}` (use `var`/`متغير`)",
                        name
                    )));
                }
                *slot = v;
                return Ok(());
            }
        }
        Err(Flow::Error(format!("cannot find `{}` to assign", name)))
    }
}
