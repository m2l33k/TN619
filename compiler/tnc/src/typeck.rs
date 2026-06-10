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
use crate::token::{array_method, is_clone_method, is_print_builtin, ArrayMethod};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    Struct(String),
    Enum(String),
    /// `[T]` — growable array.
    Vec(Box<Ty>),
    /// `Result<T, E>` — builtin success/failure type.
    Result(Box<Ty>, Box<Ty>),
    /// A side of `Result` that a bare `Ok(..)` / `Err(..)` constructor leaves
    /// undetermined; compatible with any type (resolved by context).
    Unknown,
}

/// Structural compatibility: like equality, but `Unknown` matches anything.
fn types_match(a: &Ty, b: &Ty) -> bool {
    match (a, b) {
        (Ty::Unknown, _) | (_, Ty::Unknown) => true,
        (Ty::Vec(x), Ty::Vec(y)) => types_match(x, y),
        (Ty::Result(t1, e1), Ty::Result(t2, e2)) => types_match(t1, t2) && types_match(e1, e2),
        _ => a == b,
    }
}

fn has_unknown(t: &Ty) -> bool {
    match t {
        Ty::Unknown => true,
        Ty::Vec(x) => has_unknown(x),
        Ty::Result(a, b) => has_unknown(a) || has_unknown(b),
        _ => false,
    }
}

/// Of two compatible types, keep the more concrete one (fewer `Unknown`s).
fn join(a: Ty, b: &Ty) -> Ty {
    if has_unknown(&a) {
        b.clone()
    } else {
        a
    }
}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ty::Int => write!(f, "int"),
            Ty::Float => write!(f, "f64"),
            Ty::Bool => write!(f, "bool"),
            Ty::Str => write!(f, "str"),
            Ty::Unit => write!(f, "unit"),
            Ty::Struct(n) | Ty::Enum(n) => write!(f, "{}", n),
            Ty::Vec(t) => write!(f, "[{}]", t),
            Ty::Result(t, e) => write!(f, "Result<{}, {}>", t, e),
            Ty::Unknown => write!(f, "_"),
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
    /// Mutability AND ownership (moves) are enforced here, at compile time.
    scopes: Vec<HashMap<String, Binding>>,
    cur_ret: Ty,
    /// Whether `self` in the current body is a borrow (`&self`/`&mut self`) —
    /// borrowed selves cannot be moved out of the method.
    cur_self_borrowed: bool,
}

#[derive(Clone)]
struct Binding {
    ty: Ty,
    mutable: bool,
    /// `Some(line)` once the value has been moved out of this binding.
    moved: Option<usize>,
}

impl Binding {
    fn new(ty: Ty, mutable: bool) -> Self {
        Binding {
            ty,
            mutable,
            moved: None,
        }
    }
}

/// Copy types are freely duplicated; everything else moves on use.
fn is_copy(t: &Ty) -> bool {
    matches!(t, Ty::Int | Ty::Float | Ty::Bool | Ty::Unit | Ty::Unknown)
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
            cur_self_borrowed: false,
        }
    }

    // ---- ownership / move tracking ----

    /// If `e` is a bare variable of a non-Copy type, mark it moved. Called by
    /// every consuming context (let-init, call args, struct/array literals,
    /// constructor payloads, match scrutinees, `?`, value-self receivers).
    fn mark_move(&mut self, e: &Expr) -> Result<(), String> {
        let name = match &e.kind {
            ExprKind::Ident(n) => n.clone(),
            _ => return Ok(()),
        };
        let line = e.line;
        if name == "self" && self.cur_self_borrowed {
            // Reads of fields are fine; moving the whole self is not.
            return Err(with_line(
                line,
                "cannot move `self` out of a `&self`/`&mut self` method — use `.clone()`".into(),
            ));
        }
        for scope in self.scopes.iter_mut().rev() {
            if let Some(b) = scope.get_mut(&name) {
                if !is_copy(&b.ty) {
                    b.moved = Some(line);
                }
                return Ok(());
            }
        }
        Ok(()) // not a local (e.g. a unit variant) — nothing to track
    }

    fn snapshot(&self) -> Vec<HashMap<String, Binding>> {
        self.scopes.clone()
    }

    /// Fold another control-flow branch's move-state into the current one:
    /// a value moved in EITHER branch counts as moved afterwards.
    fn merge_moves(&mut self, other: &[HashMap<String, Binding>]) {
        for (scope, oscope) in self.scopes.iter_mut().zip(other.iter()) {
            for (k, b) in scope.iter_mut() {
                if b.moved.is_none() {
                    if let Some(ob) = oscope.get(k) {
                        b.moved = ob.moved;
                    }
                }
            }
        }
    }

    /// Check a loop body, rejecting moves of variables that outlive the loop
    /// (a second iteration would use the moved value).
    fn check_loop_body(&mut self, body: &Block) -> Result<(), String> {
        let before = self.snapshot();
        self.check_block(body)?;
        for (scope, bscope) in self.scopes.iter().zip(before.iter()) {
            for (k, b) in scope.iter() {
                let was_moved = bscope.get(k).map_or(true, |ob| ob.moved.is_some());
                if let (Some(line), false) = (b.moved, was_moved) {
                    return Err(with_line(
                        line,
                        format!(
                            "`{}` is moved inside a loop — a later iteration would use \
                             the moved value; use `.clone()`",
                            k
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Resolve a source-level type to a `Ty`, using the known struct/enum names.
    fn resolve(&self, ty: &TypeExpr) -> Result<Ty, String> {
        Ok(match ty {
            TypeExpr::Array(inner) => Ty::Vec(Box::new(self.resolve(inner)?)),
            TypeExpr::Result(ok, err) => {
                Ty::Result(Box::new(self.resolve(ok)?), Box::new(self.resolve(err)?))
            }
            TypeExpr::Name(name) => match name.as_str() {
                "int" | "عدد" | "entier" => Ty::Int,
                "f64" | "float" | "عائم" | "flottant" => Ty::Float,
                "bool" | "منطقي" | "booléen" | "booleen" => Ty::Bool,
                "str" | "نص" | "chaîne" | "chaine" => Ty::Str,
                "unit" => Ty::Unit,
                other if self.struct_names.contains(other) => Ty::Struct(other.to_string()),
                other if self.enum_names.contains(other) => Ty::Enum(other.to_string()),
                other => return Err(format!("unknown type `{}`", other)),
            },
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
                    let self_ty = self.resolve(&TypeExpr::Name(b.type_name.clone()))?;
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
            // The parser canonicalized all `self` spellings to "self".
            // It is a mutable slot only inside `&mut self` methods.
            let self_mut = m.self_kind == SelfKind::MutRef;
            self.scopes
                .last_mut()
                .unwrap()
                .insert("self".into(), Binding::new(self_ty.clone(), self_mut));
        }
        self.cur_self_borrowed = matches!(m.self_kind, SelfKind::Ref | SelfKind::MutRef);
        for (p, t) in m.func.params.iter().zip(param_tys) {
            self.scopes
                .last_mut()
                .unwrap()
                .insert(p.name.clone(), Binding::new(t, false));
        }
        let body_ty = self.check_block(&m.func.body)?;
        self.scopes.pop();
        self.cur_self_borrowed = false;

        let ret = self.cur_ret.clone();
        if !types_match(&body_ty, &ret) {
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
            self.scopes
                .last_mut()
                .unwrap()
                .insert(name, Binding::new(ty, false));
        }
        let body_ty = self.check_block(&f.body)?;
        self.scopes.pop();

        let ret = self.cur_ret.clone();
        if !types_match(&body_ty, &ret) {
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
                name,
                mutable,
                ty_ann,
                init,
            } => {
                // An empty array literal has no inferable element type — the
                // annotation supplies it (`let v: [int] = []`).
                if let (Some(ann), ExprKind::ArrayLit(elems)) = (ty_ann, &init.kind) {
                    if elems.is_empty() {
                        let want = self.resolve(ann)?;
                        if let Ty::Vec(_) = want {
                            self.scopes
                                .last_mut()
                                .unwrap()
                                .insert(name.clone(), Binding::new(want, *mutable));
                            return Ok(Ty::Unit);
                        }
                    }
                }
                let init_ty = self.infer(init)?;
                let ty = match ty_ann {
                    Some(ann) => {
                        let want = self.resolve(ann)?;
                        if !types_match(&want, &init_ty) {
                            return Err(format!(
                                "let `{}`: annotated `{}` but initializer has type `{}`",
                                name, want, init_ty
                            ));
                        }
                        want
                    }
                    None => init_ty,
                };
                self.mark_move(init)?;
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(name.clone(), Binding::new(ty, *mutable));
                Ok(Ty::Unit)
            }
            Stmt::Assign { target, value } => {
                let (slot_ty, desc) = match target {
                    AssignTarget::Var(name) => (self.require_mutable(name)?, name.clone()),
                    AssignTarget::Index { name, index } => {
                        let base_ty = self.require_mutable(name)?;
                        self.expect(index, &Ty::Int, "array index")?;
                        match base_ty {
                            Ty::Vec(t) => (*t, format!("{}[..]", name)),
                            other => {
                                return Err(format!("cannot index `{}` of type `{}`", name, other))
                            }
                        }
                    }
                    AssignTarget::Field { name, field } => {
                        let base_ty = self.require_mutable(name)?;
                        match base_ty {
                            Ty::Struct(s) => {
                                let fty = self
                                    .structs
                                    .get(&s)
                                    .unwrap()
                                    .iter()
                                    .find(|(k, _)| k == field)
                                    .map(|(_, t)| t.clone())
                                    .ok_or_else(|| {
                                        format!("struct `{}` has no field `{}`", s, field)
                                    })?;
                                (fty, format!("{}.{}", name, field))
                            }
                            other => {
                                return Err(format!(
                                    "cannot assign to field `{}` on `{}`",
                                    field, other
                                ))
                            }
                        }
                    }
                };
                let val_ty = self.infer(value)?;
                if !types_match(&slot_ty, &val_ty) {
                    return Err(format!(
                        "cannot assign `{}` to `{}` of type `{}`",
                        val_ty, desc, slot_ty
                    ));
                }
                self.mark_move(value)?;
                // Assigning to a whole variable gives it a fresh value: a
                // previously-moved binding becomes usable again.
                if let AssignTarget::Var(name) = target {
                    for scope in self.scopes.iter_mut().rev() {
                        if let Some(b) = scope.get_mut(name) {
                            b.moved = None;
                            break;
                        }
                    }
                }
                Ok(Ty::Unit)
            }
            Stmt::While { cond, body } => {
                self.expect(cond, &Ty::Bool, "while condition")?;
                self.check_loop_body(body)?;
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
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(var.clone(), Binding::new(Ty::Int, false));
                let r = self.check_loop_body(body);
                self.scopes.pop();
                r?;
                Ok(Ty::Unit)
            }
            Stmt::ForEach { var, iter, body } => {
                let elem = match self.infer(iter)? {
                    Ty::Vec(t) => *t,
                    other => {
                        return Err(with_line(
                            iter.line,
                            format!("`for .. in` requires an array, got `{}`", other),
                        ))
                    }
                };
                // Iterating consumes the array (clone it to keep it).
                self.mark_move(iter)?;
                self.scopes.push(HashMap::new());
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert(var.clone(), Binding::new(elem, false));
                let r = self.check_loop_body(body);
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
                if !types_match(&t, &ret) {
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

    /// Infer the type of `e`, tagging any error with its source line (the
    /// innermost expression's line wins, since nested calls tag first).
    fn infer(&mut self, e: &Expr) -> Result<Ty, String> {
        self.infer_kind(e).map_err(|m| with_line(e.line, m))
    }

    fn infer_kind(&mut self, e: &Expr) -> Result<Ty, String> {
        match &e.kind {
            ExprKind::Int(_) => Ok(Ty::Int),
            ExprKind::Str(_) => Ok(Ty::Str),
            ExprKind::StrInterp(parts) => {
                // Each embedded expression must type-check; any displayable value
                // is allowed. The whole interpolation has type `str`.
                for part in parts {
                    if let StrPart::Expr(e) = part {
                        self.infer(e)?;
                    }
                }
                Ok(Ty::Str)
            }
            ExprKind::Bool(_) => Ok(Ty::Bool),
            ExprKind::Ident(name) => {
                if let Some(b) = self.lookup_full(name) {
                    if let Some(line) = b.moved {
                        return Err(format!(
                            "use of moved value `{}` (moved on line {}) — use `.clone()` \
                             to keep a copy",
                            name, line
                        ));
                    }
                    Ok(b.ty)
                } else if let Some(en) = self.unit_variant_enum(name) {
                    Ok(Ty::Enum(en))
                } else {
                    Err(format!("cannot find `{}` in scope", name))
                }
            }
            ExprKind::Float(_) => Ok(Ty::Float),
            ExprKind::Cast { expr, ty } => {
                let from = self.infer(expr)?;
                let to = self.resolve(ty)?;
                let numeric = |t: &Ty| *t == Ty::Int || *t == Ty::Float;
                if numeric(&from) && numeric(&to) {
                    Ok(to)
                } else {
                    Err(format!(
                        "cannot cast `{}` to `{}` (numeric casts only)",
                        from, to
                    ))
                }
            }
            ExprKind::Unary { op, rhs } => match op {
                UnOp::Neg => {
                    let t = self.infer(rhs)?;
                    if t != Ty::Int && t != Ty::Float {
                        return Err(format!("negation requires a number, got `{}`", t));
                    }
                    Ok(t)
                }
                UnOp::Not => {
                    self.expect(rhs, &Ty::Bool, "logical not")?;
                    Ok(Ty::Bool)
                }
            },
            ExprKind::Binary { op, lhs, rhs } => {
                let lt = self.infer(lhs)?;
                let rt = self.infer(rhs)?;
                use BinOp::*;
                match op {
                    Add | Sub | Mul | Div | Rem => {
                        if lt == rt && (lt == Ty::Int || lt == Ty::Float) {
                            Ok(lt)
                        } else {
                            Err(format!(
                                "arithmetic requires two ints or two floats, got `{}` and `{}`",
                                lt, rt
                            ))
                        }
                    }
                    Lt | Le | Gt | Ge => {
                        if lt == rt && (lt == Ty::Int || lt == Ty::Float || lt == Ty::Str) {
                            Ok(Ty::Bool)
                        } else {
                            return Err(format!(
                                "ordering requires two numbers or two strs, got `{}` and `{}`",
                                lt, rt
                            ));
                        }
                    }
                    Eq | Ne => {
                        if lt != rt {
                            return Err(format!("cannot compare `{}` with `{}`", lt, rt));
                        }
                        Ok(Ty::Bool)
                    }
                }
            }
            ExprKind::Call { callee, args } => {
                if is_print_builtin(callee) {
                    for a in args {
                        self.infer(a)?;
                    }
                    return Ok(Ty::Unit);
                }
                // Builtin `Result` constructors (canonicalized by the parser).
                // The other side stays `Unknown` until context resolves it.
                if callee == "Ok" || callee == "Err" {
                    if args.len() != 1 {
                        return Err(format!("`{}` takes exactly one value", callee));
                    }
                    let inner = self.infer(&args[0])?;
                    self.mark_move(&args[0])?;
                    return Ok(if callee == "Ok" {
                        Ty::Result(Box::new(inner), Box::new(Ty::Unknown))
                    } else {
                        Ty::Result(Box::new(Ty::Unknown), Box::new(inner))
                    });
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
                self.check_args(args, &sig_params, &format!("argument to `{}`", callee))?;
                Ok(sig_ret)
            }
            ExprKind::If { cond, then_b, els } => {
                self.expect(cond, &Ty::Bool, "if condition")?;
                // Each branch checks against the same starting move-state; a
                // value moved in EITHER branch is moved afterwards.
                let entry = self.snapshot();
                let then_ty = self.check_block(then_b)?;
                let after_then = self.snapshot();
                self.scopes = entry;
                match els {
                    Some(e) => {
                        let else_ty = self.infer(e)?;
                        self.merge_moves(&after_then);
                        if !types_match(&then_ty, &else_ty) {
                            return Err(format!(
                                "if branches have differing types: `{}` vs `{}`",
                                then_ty, else_ty
                            ));
                        }
                        Ok(join(then_ty, &else_ty))
                    }
                    // No else: used as a statement; its value is unit.
                    None => {
                        self.merge_moves(&after_then);
                        Ok(Ty::Unit)
                    }
                }
            }
            ExprKind::Block(b) => self.check_block(b),
            ExprKind::StructLit { name, fields } => {
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
                    self.mark_move(fexpr)?;
                }
                Ok(Ty::Struct(name.clone()))
            }
            ExprKind::Field { base, field } => {
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
            ExprKind::Path { ty, member, args } => {
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
            ExprKind::ArrayLit(elems) => {
                let mut it = elems.iter();
                let first = match it.next() {
                    Some(e) => self.infer(e)?,
                    None => {
                        return Err(
                            "cannot infer the element type of an empty array — annotate \
                             the binding, e.g. `let v: [int] = []`"
                                .into(),
                        )
                    }
                };
                for e in it {
                    self.expect(e, &first, "array element")?;
                }
                for e in elems {
                    self.mark_move(e)?;
                }
                Ok(Ty::Vec(Box::new(first)))
            }
            ExprKind::Index { base, index } => {
                let bt = self.infer(base)?;
                self.expect(index, &Ty::Int, "array index")?;
                match bt {
                    Ty::Vec(t) => Ok(*t),
                    other => Err(format!("cannot index a value of type `{}`", other)),
                }
            }
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                let rt = self.infer(receiver)?;
                // Builtin array methods: len/طول/longueur, push/أضف/ajoute,
                // pop/اسحب/retire.
                if let Ty::Vec(elem) = &rt {
                    if let Some(m) = array_method(method) {
                        return match m {
                            ArrayMethod::Len => {
                                self.check_args(args, &[], "len")?;
                                Ok(Ty::Int)
                            }
                            ArrayMethod::Push => {
                                self.check_args(args, &[(**elem).clone()], "push")?;
                                self.require_mutable_receiver(receiver, "push")?;
                                Ok(Ty::Unit)
                            }
                            ArrayMethod::Pop => {
                                self.check_args(args, &[], "pop")?;
                                self.require_mutable_receiver(receiver, "pop")?;
                                Ok((**elem).clone())
                            }
                        };
                    }
                }
                // Builtin `.clone()` — works on any value, never moves the
                // receiver. A user-defined `clone` method takes priority.
                if is_clone_method(method) && args.is_empty() {
                    let has_user_clone = match &rt {
                        Ty::Struct(n) | Ty::Enum(n) => self.method_sig(n, method).is_some(),
                        _ => false,
                    };
                    if !has_user_clone {
                        return Ok(rt);
                    }
                }
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
                if self_kind == SelfKind::MutRef {
                    self.require_mutable_receiver(
                        receiver,
                        &format!("`&mut self` method `{}`", method),
                    )?;
                }
                if self_kind == SelfKind::Value {
                    // A by-value method consumes its receiver.
                    self.mark_move(receiver)?;
                }
                self.check_args(args, &params, &format!("method `{}`", method))?;
                Ok(ret)
            }
            ExprKind::Match { scrutinee, arms } => self.infer_match(scrutinee, arms),
            ExprKind::Try(inner) => {
                let it = self.infer(inner)?;
                self.mark_move(inner)?;
                let (ok_ty, err_ty) = match it {
                    Ty::Result(t, e) => (*t, *e),
                    other => {
                        return Err(format!("`?` requires a `Result<_, _>`, found `{}`", other))
                    }
                };
                match &self.cur_ret {
                    Ty::Result(_, ret_err) => {
                        if !types_match(&err_ty, ret_err) {
                            return Err(format!(
                                "`?` propagates `{}` but the function returns `Result<_, {}>`",
                                err_ty, ret_err
                            ));
                        }
                        Ok(ok_ty)
                    }
                    other => Err(format!(
                        "`?` can only be used in a function returning `Result`, \
                         but this one returns `{}`",
                        other
                    )),
                }
            }
        }
    }

    fn infer_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> Result<Ty, String> {
        if arms.is_empty() {
            return Err("match must have at least one arm".into());
        }
        let scrut_ty = self.infer(scrutinee)?;
        // Matching on a value consumes it (bindings may move pieces out).
        self.mark_move(scrutinee)
            .map_err(|m| with_line(scrutinee.line, m))?;
        let mut arm_ty: Option<Ty> = None;
        let mut has_wildcard = false;
        let mut covered_variants: HashSet<String> = HashSet::new();
        let mut covered_bools: HashSet<bool> = HashSet::new();

        // Arms are alternative branches: each starts from the same move-state,
        // and their moves are unioned afterwards.
        let entry = self.snapshot();
        let mut arm_states: Vec<Vec<HashMap<String, Binding>>> = Vec::new();

        for arm in arms {
            self.scopes = entry.clone();
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
            arm_states.push(self.snapshot());
            let body_ty = body_ty?;
            match &arm_ty {
                None => arm_ty = Some(body_ty),
                Some(t) if !types_match(t, &body_ty) => {
                    return Err(format!(
                        "match arms have differing types: `{}` vs `{}`",
                        t, body_ty
                    ));
                }
                Some(t) => arm_ty = Some(join(t.clone(), &body_ty)),
            }
        }
        self.scopes = entry;
        for st in &arm_states {
            self.merge_moves(st);
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
                Ty::Result(..) => {
                    let missing: Vec<&str> = ["Ok", "Err"]
                        .into_iter()
                        .filter(|v| !covered_variants.contains(*v))
                        .collect();
                    if !missing.is_empty() {
                        return Err(format!(
                            "non-exhaustive match on `Result`: missing variant(s) {} — add an arm for each, or a `_` arm",
                            missing.join(", ")
                        ));
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
                        .insert(name.clone(), Binding::new(ty.clone(), false));
                    Ok(Coverage::Wildcard)
                }
            }
            Pattern::Variant {
                enum_name,
                name,
                subs,
            } => {
                // Builtin `Result` patterns: `Ok(x)` / `Err(e)`.
                if let Ty::Result(ok_ty, err_ty) = ty {
                    let payload = match name.as_str() {
                        "Ok" => ok_ty,
                        "Err" => err_ty,
                        other => {
                            return Err(format!(
                                "`Result` has no variant `{}` (expected Ok/Err)",
                                other
                            ))
                        }
                    };
                    if subs.len() != 1 {
                        return Err(format!("`{}` pattern binds exactly one value", name));
                    }
                    self.bind_pattern(&subs[0], payload)?;
                    return Ok(Coverage::Variant(name.clone()));
                }
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
            self.mark_move(a)?;
        }
        Ok(Ty::Enum(enum_name.to_string()))
    }

    // ---- helpers ----

    /// A mutating call needs its receiver to be a mutable place — a mutable
    /// variable or a field of one (value semantics: mutating a temporary
    /// would silently do nothing).
    fn require_mutable_receiver(&self, receiver: &Expr, what: &str) -> Result<(), String> {
        match &receiver.kind {
            ExprKind::Ident(name) => {
                self.require_mutable(name)?;
                Ok(())
            }
            ExprKind::Field { base, .. } if matches!(base.kind, ExprKind::Ident(_)) => {
                if let ExprKind::Ident(name) = &base.kind {
                    self.require_mutable(name)?;
                }
                Ok(())
            }
            _ => Err(format!(
                "{} requires a mutable variable (or a field of one) as receiver",
                what
            )),
        }
    }

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
            self.mark_move(a)?; // passing an argument gives it away
        }
        Ok(())
    }

    fn expect(&mut self, e: &Expr, want: &Ty, ctx: &str) -> Result<(), String> {
        // An empty array literal takes its element type from the expected
        // type (struct fields, function args, enum payloads, ...).
        if let (ExprKind::ArrayLit(elems), Ty::Vec(_)) = (&e.kind, want) {
            if elems.is_empty() {
                return Ok(());
            }
        }
        let got = self.infer(e)?;
        self.expect_ty(&got, want, ctx)
            .map_err(|m| with_line(e.line, m))
    }

    fn expect_ty(&self, got: &Ty, want: &Ty, ctx: &str) -> Result<(), String> {
        if types_match(got, want) {
            Ok(())
        } else {
            Err(format!("{}: expected `{}`, found `{}`", ctx, want, got))
        }
    }

    fn lookup_full(&self, name: &str) -> Option<Binding> {
        self.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }

    /// Resolve a variable that is about to be mutated; immutability is a
    /// compile error (trilingual hint included). Whole-variable assignment
    /// may target a moved binding (it revives it), so moves are not checked
    /// here — consuming contexts check them via `infer`.
    fn require_mutable(&self, name: &str) -> Result<Ty, String> {
        match self.lookup_full(name) {
            None => Err(format!("cannot assign to unknown variable `{}`", name)),
            Some(Binding { mutable: false, .. }) if name == "self" => {
                Err("cannot mutate `self` here — declare the method with `&mut self`".into())
            }
            Some(Binding { mutable: false, .. }) => Err(format!(
                "cannot mutate immutable binding `{}` — declare it with `var`/`متغير`/`variable`",
                name
            )),
            Some(b) => Ok(b.ty),
        }
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

/// Prefix a diagnostic with its source line, unless it already carries one.
fn with_line(line: usize, msg: String) -> String {
    if msg.starts_with("line ") {
        msg
    } else {
        format!("line {}: {}", line, msg)
    }
}
