//! Static type checker.
//!
//! Runs after parsing and before execution. Catches type errors up front and,
//! crucially, performs **compile-time match exhaustiveness checking** — the
//! safety feature the interpreter alone could only enforce at runtime.
//!
//! Inference is local: `let` bindings infer their type from the initializer,
//! while function signatures, struct fields, and enum payloads are explicit
//! anchors (per the TN619 design).

use crate::ast::*;
use crate::token::is_print_builtin;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int,
    Bool,
    Str,
    Unit,
    Struct(String),
    Enum(String),
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Int => write!(f, "int"),
            Ty::Bool => write!(f, "bool"),
            Ty::Str => write!(f, "str"),
            Ty::Unit => write!(f, "unit"),
            Ty::Struct(n) | Ty::Enum(n) => write!(f, "{}", n),
        }
    }
}

struct FnSig {
    params: Vec<Ty>,
    ret: Ty,
}

struct MethodSig {
    self_kind: SelfKind,
    params: Vec<Ty>,
    ret: Ty,
}

/// What a top-level match pattern covers, for exhaustiveness analysis.
enum Coverage {
    Wildcard,        // `_` or a bare binding — covers everything
    Variant(String), // an enum variant
    Bool(bool),      // a boolean literal
    Partial,         // an int/str literal — never exhaustive on its own
}

pub struct Checker {
    struct_names: HashSet<String>,
    enum_names: HashSet<String>,
    structs: HashMap<String, Vec<(String, Ty)>>,
    enums: HashMap<String, Vec<(String, Vec<Ty>)>>,
    variant_to_enum: HashMap<String, String>,
    funcs: HashMap<String, FnSig>,
    methods: HashMap<(String, String), MethodSig>,
    scopes: Vec<HashMap<String, Ty>>,
    cur_ret: Ty,
}

impl Checker {
    pub fn new() -> Self {
        Checker {
            struct_names: HashSet::new(),
            enum_names: HashSet::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            variant_to_enum: HashMap::new(),
            funcs: HashMap::new(),
            methods: HashMap::new(),
            scopes: vec![],
            cur_ret: Ty::Unit,
        }
    }

    /// Resolve a type-name string to a `Ty`, using the known struct/enum names.
    fn resolve(&self, name: &str) -> Result<Ty, String> {
        Ok(match name {
            "int" | "عدد" => Ty::Int,
            "bool" | "منطقي" => Ty::Bool,
            "str" | "نص" => Ty::Str,
            "unit" => Ty::Unit,
            other if self.struct_names.contains(other) => Ty::Struct(other.to_string()),
            other if self.enum_names.contains(other) => Ty::Enum(other.to_string()),
            other => return Err(format!("unknown type `{}`", other)),
        })
    }

    pub fn check(&mut self, prog: &Program) -> Result<(), String> {
        // Pass 1: gather all user-defined type names (so forward references and
        // mutually-recursive types resolve).
        for item in &prog.items {
            match item {
                Item::Struct(s) => {
                    if !self.struct_names.insert(s.name.clone())
                        || self.enum_names.contains(&s.name)
                    {
                        return Err(format!("type `{}` is declared more than once", s.name));
                    }
                }
                Item::Enum(e) => {
                    if !self.enum_names.insert(e.name.clone())
                        || self.struct_names.contains(&e.name)
                    {
                        return Err(format!("type `{}` is declared more than once", e.name));
                    }
                }
                Item::Fn(_) | Item::Impl(_) => {}
            }
        }

        // Pass 2: resolve struct fields, enum variants, function signatures.
        for item in &prog.items {
            match item {
                Item::Struct(s) => {
                    let mut fields = Vec::new();
                    for (fname, fty) in &s.fields {
                        fields.push((fname.clone(), self.resolve(fty)?));
                    }
                    self.structs.insert(s.name.clone(), fields);
                }
                Item::Enum(e) => {
                    let mut variants = Vec::new();
                    for v in &e.variants {
                        if let Some(other) = self.variant_to_enum.get(&v.name) {
                            return Err(format!(
                                "variant `{}` declared in both `{}` and `{}`",
                                v.name, other, e.name
                            ));
                        }
                        self.variant_to_enum.insert(v.name.clone(), e.name.clone());
                        let mut payloads = Vec::new();
                        for p in &v.payloads {
                            payloads.push(self.resolve(p)?);
                        }
                        variants.push((v.name.clone(), payloads));
                    }
                    self.enums.insert(e.name.clone(), variants);
                }
                Item::Fn(f) => {
                    let mut params = Vec::new();
                    for p in &f.params {
                        params.push(self.resolve(&p.ty)?);
                    }
                    let ret = match &f.ret {
                        Some(t) => self.resolve(t)?,
                        None => Ty::Unit,
                    };
                    self.funcs.insert(f.name.clone(), FnSig { params, ret });
                }
                Item::Impl(b) => {
                    if !self.struct_names.contains(&b.type_name)
                        && !self.enum_names.contains(&b.type_name)
                    {
                        return Err(format!("cannot impl unknown type `{}`", b.type_name));
                    }
                    for m in &b.methods {
                        let mut params = Vec::new();
                        for p in &m.func.params {
                            params.push(self.resolve(&p.ty)?);
                        }
                        let ret = match &m.func.ret {
                            Some(t) => self.resolve(t)?,
                            None => Ty::Unit,
                        };
                        self.methods.insert(
                            (b.type_name.clone(), m.func.name.clone()),
                            MethodSig {
                                self_kind: m.self_kind,
                                params,
                                ret,
                            },
                        );
                    }
                }
            }
        }

        // Pass 3: check function and method bodies.
        for item in &prog.items {
            match item {
                Item::Fn(f) => self.check_fn(f)?,
                Item::Impl(b) => {
                    let self_ty = self.resolve(&b.type_name)?;
                    for m in &b.methods {
                        self.check_method(&self_ty, m)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_method(&mut self, self_ty: &Ty, m: &MethodDecl) -> Result<(), String> {
        let sig = self
            .methods
            .get(&(self_ty.to_string(), m.func.name.clone()))
            .unwrap();
        self.cur_ret = sig.ret.clone();
        let param_tys = sig.params.clone();

        self.scopes.push(HashMap::new());
        if m.self_kind != SelfKind::None {
            // `self` is visible under both spellings inside the body.
            self.scopes
                .last_mut()
                .unwrap()
                .insert("self".into(), self_ty.clone());
            self.scopes
                .last_mut()
                .unwrap()
                .insert("الذات".into(), self_ty.clone());
        }
        for (p, t) in m.func.params.iter().zip(param_tys) {
            self.scopes.last_mut().unwrap().insert(p.name.clone(), t);
        }
        let body_ty = self.check_block(&m.func.body)?;
        self.scopes.pop();

        let ret = self.cur_ret.clone();
        if body_ty != ret {
            return Err(format!(
                "method `{}`: body has type `{}` but the declared return type is `{}`",
                m.func.name, body_ty, ret
            ));
        }
        Ok(())
    }

    fn check_fn(&mut self, f: &FnDecl) -> Result<(), String> {
        let sig = self.funcs.get(&f.name).unwrap();
        self.cur_ret = sig.ret.clone();
        let params: Vec<(String, Ty)> = f
            .params
            .iter()
            .zip(sig.params.clone())
            .map(|(p, t)| (p.name.clone(), t))
            .collect();

        self.scopes.push(HashMap::new());
        for (name, ty) in params {
            self.scopes.last_mut().unwrap().insert(name, ty);
        }
        let body_ty = self.check_block(&f.body)?;
        self.scopes.pop();

        let ret = self.cur_ret.clone();
        if body_ty != ret {
            return Err(format!(
                "function `{}`: body has type `{}` but the declared return type is `{}`",
                f.name, body_ty, ret
            ));
        }
        Ok(())
    }

    /// A block's type is the type of its final statement if that statement is a
    /// bare expression, otherwise unit.
    fn check_block(&mut self, b: &Block) -> Result<Ty, String> {
        self.scopes.push(HashMap::new());
        let mut last = Ty::Unit;
        for s in &b.stmts {
            match self.check_stmt(s) {
                Ok(t) => last = t,
                Err(e) => {
                    self.scopes.pop();
                    return Err(e);
                }
            }
        }
        self.scopes.pop();
        Ok(last)
    }

    fn check_stmt(&mut self, s: &Stmt) -> Result<Ty, String> {
        match s {
            Stmt::Let {
                name, ty_ann, init, ..
            } => {
                let init_ty = self.infer(init)?;
                let ty = match ty_ann {
                    Some(ann) => {
                        let want = self.resolve(ann)?;
                        if want != init_ty {
                            return Err(format!(
                                "let `{}`: annotated `{}` but initializer has type `{}`",
                                name, want, init_ty
                            ));
                        }
                        want
                    }
                    None => init_ty,
                };
                self.scopes.last_mut().unwrap().insert(name.clone(), ty);
                Ok(Ty::Unit)
            }
            Stmt::Assign { name, value } => {
                let var_ty = self
                    .lookup(name)
                    .ok_or_else(|| format!("cannot assign to unknown variable `{}`", name))?;
                let val_ty = self.infer(value)?;
                if var_ty != val_ty {
                    return Err(format!(
                        "cannot assign `{}` to `{}` of type `{}`",
                        val_ty, name, var_ty
                    ));
                }
                Ok(Ty::Unit)
            }
            Stmt::While { cond, body } => {
                self.expect(cond, &Ty::Bool, "while condition")?;
                self.check_block(body)?;
                Ok(Ty::Unit)
            }
            Stmt::For {
                var,
                start,
                end,
                body,
            } => {
                self.expect(start, &Ty::Int, "for range start")?;
                self.expect(end, &Ty::Int, "for range end")?;
                self.scopes.push(HashMap::new());
                self.scopes.last_mut().unwrap().insert(var.clone(), Ty::Int);
                let r = self.check_block(body);
                self.scopes.pop();
                r?;
                Ok(Ty::Unit)
            }
            Stmt::Return(opt) => {
                let t = match opt {
                    Some(e) => self.infer(e)?,
                    None => Ty::Unit,
                };
                let ret = self.cur_ret.clone();
                if t != ret {
                    return Err(format!(
                        "return type `{}` does not match declared `{}`",
                        t, ret
                    ));
                }
                Ok(Ty::Unit)
            }
            Stmt::Expr(e) => self.infer(e),
        }
    }

    fn infer(&mut self, e: &Expr) -> Result<Ty, String> {
        match e {
            Expr::Int(_) => Ok(Ty::Int),
            Expr::Str(_) => Ok(Ty::Str),
            Expr::StrInterp(parts) => {
                // Each embedded expression must type-check; any displayable value
                // is allowed. The whole interpolation has type `str`.
                for part in parts {
                    if let StrPart::Expr(e) = part {
                        self.infer(e)?;
                    }
                }
                Ok(Ty::Str)
            }
            Expr::Bool(_) => Ok(Ty::Bool),
            Expr::Ident(name) => {
                if let Some(t) = self.lookup(name) {
                    Ok(t)
                } else if let Some(en) = self.unit_variant_enum(name) {
                    Ok(Ty::Enum(en))
                } else {
                    Err(format!("cannot find `{}` in scope", name))
                }
            }
            Expr::Unary { op, rhs } => match op {
                UnOp::Neg => {
                    self.expect(rhs, &Ty::Int, "negation")?;
                    Ok(Ty::Int)
                }
                UnOp::Not => {
                    self.expect(rhs, &Ty::Bool, "logical not")?;
                    Ok(Ty::Bool)
                }
            },
            Expr::Binary { op, lhs, rhs } => {
                let lt = self.infer(lhs)?;
                let rt = self.infer(rhs)?;
                use BinOp::*;
                match op {
                    Add | Sub | Mul | Div | Rem => {
                        if lt != Ty::Int || rt != Ty::Int {
                            return Err(format!(
                                "arithmetic requires int, got `{}` and `{}`",
                                lt, rt
                            ));
                        }
                        Ok(Ty::Int)
                    }
                    Lt | Le | Gt | Ge => {
                        if !(lt == rt && (lt == Ty::Int || lt == Ty::Str)) {
                            return Err(format!(
                                "ordering requires two ints or two strs, got `{}` and `{}`",
                                lt, rt
                            ));
                        }
                        Ok(Ty::Bool)
                    }
                    Eq | Ne => {
                        if lt != rt {
                            return Err(format!("cannot compare `{}` with `{}`", lt, rt));
                        }
                        Ok(Ty::Bool)
                    }
                }
            }
            Expr::Call { callee, args } => {
                if is_print_builtin(callee) {
                    for a in args {
                        self.infer(a)?;
                    }
                    return Ok(Ty::Unit);
                }
                // Bare tuple-variant construction (e.g. `Circle(r)`).
                if let Some(en) = self.variant_to_enum.get(callee).cloned() {
                    return self.check_variant_construction(&en, callee, args);
                }
                let sig_params;
                let sig_ret;
                {
                    let sig = self
                        .funcs
                        .get(callee)
                        .ok_or_else(|| format!("cannot find function `{}`", callee))?;
                    sig_params = sig.params.clone();
                    sig_ret = sig.ret.clone();
                }
                if args.len() != sig_params.len() {
                    return Err(format!(
                        "function `{}` expects {} arg(s), got {}",
                        callee,
                        sig_params.len(),
                        args.len()
                    ));
                }
                for (a, want) in args.iter().zip(sig_params.iter()) {
                    self.expect(a, want, &format!("argument to `{}`", callee))?;
                }
                Ok(sig_ret)
            }
            Expr::If { cond, then_b, els } => {
                self.expect(cond, &Ty::Bool, "if condition")?;
                let then_ty = self.check_block(then_b)?;
                match els {
                    Some(e) => {
                        let else_ty = self.infer(e)?;
                        if then_ty != else_ty {
                            return Err(format!(
                                "if branches have differing types: `{}` vs `{}`",
                                then_ty, else_ty
                            ));
                        }
                        Ok(then_ty)
                    }
                    // No else: used as a statement; its value is unit.
                    None => Ok(Ty::Unit),
                }
            }
            Expr::Block(b) => self.check_block(b),
            Expr::StructLit { name, fields } => {
                let decl = self
                    .structs
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("unknown struct `{}`", name))?;
                if fields.len() != decl.len() {
                    return Err(format!(
                        "struct `{}` expects {} field(s), got {}",
                        name,
                        decl.len(),
                        fields.len()
                    ));
                }
                for (fname, fexpr) in fields {
                    let want = decl
                        .iter()
                        .find(|(k, _)| k == fname)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| format!("struct `{}` has no field `{}`", name, fname))?;
                    self.expect(fexpr, &want, &format!("field `{}`", fname))?;
                }
                Ok(Ty::Struct(name.clone()))
            }
            Expr::Field { base, field } => {
                let bt = self.infer(base)?;
                match bt {
                    Ty::Struct(s) => self
                        .structs
                        .get(&s)
                        .unwrap()
                        .iter()
                        .find(|(k, _)| k == field)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| format!("struct `{}` has no field `{}`", s, field)),
                    other => Err(format!("cannot access field `{}` on `{}`", field, other)),
                }
            }
            Expr::Path { ty, member, args } => {
                // Enum construction: `Shape::Circle(..)` / `Color::Red`.
                if self.enum_names.contains(ty)
                    && self.enums.get(ty).unwrap().iter().any(|(v, _)| v == member)
                {
                    return self.check_variant_construction(ty, member, args);
                }
                // Associated function: `Point::new(..)`.
                if let Some((self_kind, params, ret)) = self.method_sig(ty, member) {
                    if self_kind != SelfKind::None {
                        return Err(format!(
                            "`{}::{}` is a method; call it on a value with `.`",
                            ty, member
                        ));
                    }
                    self.check_args(args, &params, &format!("`{}::{}`", ty, member))?;
                    return Ok(ret);
                }
                Err(format!("no `{}::{}`", ty, member))
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
            } => {
                let rt = self.infer(receiver)?;
                let tyname = match &rt {
                    Ty::Struct(n) | Ty::Enum(n) => n.clone(),
                    other => return Err(format!("type `{}` has no methods", other)),
                };
                let (self_kind, params, ret) = self
                    .method_sig(&tyname, method)
                    .ok_or_else(|| format!("`{}` has no method `{}`", tyname, method))?;
                if self_kind == SelfKind::None {
                    return Err(format!(
                        "`{}::{}` is an associated function; call it as `{}::{}(..)`",
                        tyname, method, tyname, method
                    ));
                }
                self.check_args(args, &params, &format!("method `{}`", method))?;
                Ok(ret)
            }
            Expr::Match { scrutinee, arms } => self.infer_match(scrutinee, arms),
        }
    }

    fn infer_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> Result<Ty, String> {
        if arms.is_empty() {
            return Err("match must have at least one arm".into());
        }
        let scrut_ty = self.infer(scrutinee)?;
        let mut arm_ty: Option<Ty> = None;
        let mut has_wildcard = false;
        let mut covered_variants: HashSet<String> = HashSet::new();
        let mut covered_bools: HashSet<bool> = HashSet::new();

        for arm in arms {
            self.scopes.push(HashMap::new());
            let cov = self.bind_pattern(&arm.pattern, &scrut_ty);
            let cov = match cov {
                Ok(c) => c,
                Err(e) => {
                    self.scopes.pop();
                    return Err(e);
                }
            };
            match cov {
                Coverage::Wildcard => has_wildcard = true,
                Coverage::Variant(v) => {
                    covered_variants.insert(v);
                }
                Coverage::Bool(b) => {
                    covered_bools.insert(b);
                }
                Coverage::Partial => {}
            }
            let body_ty = self.infer(&arm.body);
            self.scopes.pop();
            let body_ty = body_ty?;
            match &arm_ty {
                None => arm_ty = Some(body_ty),
                Some(t) if *t != body_ty => {
                    return Err(format!(
                        "match arms have differing types: `{}` vs `{}`",
                        t, body_ty
                    ));
                }
                _ => {}
            }
        }

        // Exhaustiveness.
        if !has_wildcard {
            match &scrut_ty {
                Ty::Enum(e) => {
                    let all: Vec<String> = self
                        .enums
                        .get(e)
                        .unwrap()
                        .iter()
                        .map(|(n, _)| n.clone())
                        .collect();
                    let missing: Vec<String> = all
                        .into_iter()
                        .filter(|v| !covered_variants.contains(v))
                        .collect();
                    if !missing.is_empty() {
                        return Err(format!(
                            "non-exhaustive match on `{}`: missing variant(s) {} — add an arm for each, or a `_` arm",
                            e,
                            missing.join(", ")
                        ));
                    }
                }
                Ty::Bool => {
                    if !(covered_bools.contains(&true) && covered_bools.contains(&false)) {
                        return Err(
                            "non-exhaustive match on `bool`: cover both `true` and `false`, or add a `_` arm".into(),
                        );
                    }
                }
                other => {
                    return Err(format!(
                        "non-exhaustive match on `{}`: add a `_` (wildcard) arm",
                        other
                    ));
                }
            }
        }

        Ok(arm_ty.unwrap())
    }

    /// Type-checks a pattern against `ty`, binding pattern variables into the
    /// current scope, and reports what the top-level pattern covers.
    fn bind_pattern(&mut self, pat: &Pattern, ty: &Ty) -> Result<Coverage, String> {
        match pat {
            Pattern::Wildcard => Ok(Coverage::Wildcard),
            Pattern::Int(_) => {
                self.expect_ty(ty, &Ty::Int, "int pattern")?;
                Ok(Coverage::Partial)
            }
            Pattern::Str(_) => {
                self.expect_ty(ty, &Ty::Str, "str pattern")?;
                Ok(Coverage::Partial)
            }
            Pattern::Bool(b) => {
                self.expect_ty(ty, &Ty::Bool, "bool pattern")?;
                Ok(Coverage::Bool(*b))
            }
            Pattern::Ident(name) => {
                // A bare name that is a unit variant matches that variant;
                // otherwise it binds the whole value.
                if let Some(en) = self.unit_variant_enum(name) {
                    self.expect_ty(ty, &Ty::Enum(en), "variant pattern")?;
                    Ok(Coverage::Variant(name.clone()))
                } else {
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), ty.clone());
                    Ok(Coverage::Wildcard)
                }
            }
            Pattern::Variant {
                enum_name,
                name,
                subs,
            } => {
                let en = match ty {
                    Ty::Enum(e) => e.clone(),
                    other => {
                        return Err(format!(
                            "variant pattern `{}` used on non-enum `{}`",
                            name, other
                        ))
                    }
                };
                if let Some(expected) = enum_name {
                    if expected != &en {
                        return Err(format!(
                            "variant `{}::{}` does not match enum `{}`",
                            expected, name, en
                        ));
                    }
                }
                let payloads = self
                    .enums
                    .get(&en)
                    .unwrap()
                    .iter()
                    .find(|(v, _)| v == name)
                    .map(|(_, p)| p.clone())
                    .ok_or_else(|| format!("enum `{}` has no variant `{}`", en, name))?;
                if subs.len() != payloads.len() {
                    return Err(format!(
                        "variant `{}` binds {} value(s) but has {}",
                        name,
                        subs.len(),
                        payloads.len()
                    ));
                }
                for (sub, pty) in subs.iter().zip(payloads.iter()) {
                    self.bind_pattern(sub, pty)?;
                }
                Ok(Coverage::Variant(name.clone()))
            }
        }
    }

    fn check_variant_construction(
        &mut self,
        enum_name: &str,
        variant: &str,
        args: &[Expr],
    ) -> Result<Ty, String> {
        let payloads = self
            .enums
            .get(enum_name)
            .ok_or_else(|| format!("unknown enum `{}`", enum_name))?
            .iter()
            .find(|(v, _)| v == variant)
            .map(|(_, p)| p.clone())
            .ok_or_else(|| format!("enum `{}` has no variant `{}`", enum_name, variant))?;
        if args.len() != payloads.len() {
            return Err(format!(
                "variant `{}` expects {} value(s), got {}",
                variant,
                payloads.len(),
                args.len()
            ));
        }
        for (a, want) in args.iter().zip(payloads.iter()) {
            self.expect(a, want, &format!("payload of `{}`", variant))?;
        }
        Ok(Ty::Enum(enum_name.to_string()))
    }

    // ---- helpers ----

    fn method_sig(&self, ty: &str, name: &str) -> Option<(SelfKind, Vec<Ty>, Ty)> {
        self.methods
            .get(&(ty.to_string(), name.to_string()))
            .map(|s| (s.self_kind, s.params.clone(), s.ret.clone()))
    }

    fn check_args(&mut self, args: &[Expr], params: &[Ty], ctx: &str) -> Result<(), String> {
        if args.len() != params.len() {
            return Err(format!(
                "{} expects {} arg(s), got {}",
                ctx,
                params.len(),
                args.len()
            ));
        }
        for (a, want) in args.iter().zip(params.iter()) {
            self.expect(a, want, ctx)?;
        }
        Ok(())
    }

    fn expect(&mut self, e: &Expr, want: &Ty, ctx: &str) -> Result<(), String> {
        let got = self.infer(e)?;
        self.expect_ty(&got, want, ctx)
    }

    fn expect_ty(&self, got: &Ty, want: &Ty, ctx: &str) -> Result<(), String> {
        if got == want {
            Ok(())
        } else {
            Err(format!("{}: expected `{}`, found `{}`", ctx, want, got))
        }
    }

    fn lookup(&self, name: &str) -> Option<Ty> {
        self.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }

    fn unit_variant_enum(&self, name: &str) -> Option<String> {
        let en = self.variant_to_enum.get(name)?;
        let payloads = &self.enums.get(en)?.iter().find(|(v, _)| v == name)?.1;
        if payloads.is_empty() {
            Some(en.clone())
        } else {
            None
        }
    }
}
