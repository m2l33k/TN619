//! Native backend (Cranelift JIT) — `tnc jit <file.tn>`.
//!
//! Compiles the **int/bool subset** of TN619 straight to machine code:
//! functions over `int`/`bool`/unit, arithmetic, comparisons, `let`/`var`,
//! assignment, `if`/`else` (statement and expression), `while`, `for i in
//! a..b`, recursion, and `print` of integers/bools. Anything outside the
//! subset reports a clean "not yet supported" error — `tnc run` (the
//! interpreter) remains the reference backend for the full language.
//!
//! The trilingual surface needs nothing special here: by this stage the
//! program is a language-neutral, type-checked AST.

use crate::ast::*;
use crate::token::is_print_builtin;
use cranelift_codegen::ir::{types, AbiParam, InstBuilder, Value};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;

// ---- runtime symbols the JIT'd code calls back into ----

extern "C" fn tn_print_int(v: i64) {
    print!("{}", v);
}
extern "C" fn tn_print_sp() {
    print!(" ");
}
extern "C" fn tn_print_nl() {
    println!();
}

type JResult<T> = Result<T, String>;

fn unsupported(what: &str) -> String {
    format!(
        "the native backend does not support {} yet — run this program with `tnc run` instead",
        what
    )
}

/// JIT-compile and execute the program's entry point.
pub fn jit_run(prog: &Program) -> JResult<()> {
    let mut flags = settings::builder();
    flags
        .set("use_colocated_libcalls", "false")
        .map_err(|e| e.to_string())?;
    flags.set("is_pic", "false").map_err(|e| e.to_string())?;
    let isa = cranelift_native::builder()
        .map_err(|e| format!("host machine is not supported by the JIT: {}", e))?
        .finish(settings::Flags::new(flags))
        .map_err(|e| e.to_string())?;

    let mut jb = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    jb.symbol("tn_print_int", tn_print_int as *const u8);
    jb.symbol("tn_print_sp", tn_print_sp as *const u8);
    jb.symbol("tn_print_nl", tn_print_nl as *const u8);
    let mut module = JITModule::new(jb);

    // Only plain functions are in the subset (no structs/impls).
    let mut fns: Vec<&FnDecl> = Vec::new();
    for item in &prog.items {
        match item {
            Item::Fn(f) => fns.push(f),
            Item::Struct(_) | Item::Enum(_) | Item::Impl(_) => {
                return Err(unsupported("structs, enums, or impl blocks"))
            }
        }
    }

    // Runtime print helpers.
    let mut sig = module.make_signature();
    sig.params.push(AbiParam::new(types::I64));
    let print_int = module
        .declare_function("tn_print_int", Linkage::Import, &sig)
        .map_err(|e| e.to_string())?;
    let sig_void = module.make_signature();
    let print_sp = module
        .declare_function("tn_print_sp", Linkage::Import, &sig_void)
        .map_err(|e| e.to_string())?;
    let print_nl = module
        .declare_function("tn_print_nl", Linkage::Import, &sig_void)
        .map_err(|e| e.to_string())?;

    // Pass 1: declare every function (so calls and recursion resolve).
    let mut ids: HashMap<String, (FuncId, usize, bool)> = HashMap::new();
    for f in &fns {
        let mut sig = module.make_signature();
        for p in &f.params {
            check_subset_type(Some(&p.ty))?;
            sig.params.push(AbiParam::new(types::I64));
        }
        let has_ret = match &f.ret {
            None => false,
            Some(t) => {
                check_subset_type(Some(t))?;
                true
            }
        };
        if has_ret {
            sig.returns.push(AbiParam::new(types::I64));
        }
        let id = module
            .declare_function(&f.name, Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;
        ids.insert(f.name.clone(), (id, f.params.len(), has_ret));
    }

    // Pass 2: compile bodies.
    let mut ctx = module.make_context();
    let mut fb_ctx = FunctionBuilderContext::new();
    for f in &fns {
        let (id, _, has_ret) = ids[&f.name];
        module.clear_context(&mut ctx);
        for _ in &f.params {
            ctx.func.signature.params.push(AbiParam::new(types::I64));
        }
        if has_ret {
            ctx.func.signature.returns.push(AbiParam::new(types::I64));
        }

        {
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);
            let entry = builder.create_block();
            builder.append_block_params_for_function_params(entry);
            builder.switch_to_block(entry);
            builder.seal_block(entry);

            let mut gen = FnGen {
                module: &mut module,
                builder,
                ids: &ids,
                vars: vec![HashMap::new()],
                print_int,
                print_sp,
                print_nl,
            };
            for (i, p) in f.params.iter().enumerate() {
                let v = gen.builder.block_params(entry)[i];
                let var = gen.new_var(&p.name);
                gen.builder.def_var(var, v);
            }
            let tail = gen.gen_block(&f.body)?;
            if has_ret {
                let v = tail.ok_or_else(|| {
                    format!("function `{}`: missing a tail value for the JIT", f.name)
                })?;
                gen.builder.ins().return_(&[v]);
            } else {
                gen.builder.ins().return_(&[]);
            }
            gen.builder.finalize();
        }

        module
            .define_function(id, &mut ctx)
            .map_err(|e| e.to_string())?;
    }
    module.finalize_definitions().map_err(|e| e.to_string())?;

    // Entry point, in any of the three surface spellings.
    let main = ["main", "رئيسي", "principal"]
        .iter()
        .find_map(|n| ids.get(*n))
        .ok_or_else(|| "no entry point: define `fn main()`".to_string())?;
    let ptr = module.get_finalized_function(main.0);
    if main.1 != 0 {
        return Err("the entry point must take no parameters".into());
    }
    unsafe {
        if main.2 {
            let f: extern "C" fn() -> i64 = std::mem::transmute(ptr);
            f();
        } else {
            let f: extern "C" fn() = std::mem::transmute(ptr);
            f();
        }
    }
    Ok(())
}

/// Only `int` and `bool` (any spelling) exist in the JIT subset; both are I64.
fn check_subset_type(t: Option<&TypeExpr>) -> JResult<()> {
    match t {
        None => Ok(()),
        Some(TypeExpr::Name(n)) => match n.as_str() {
            "int" | "عدد" | "entier" | "bool" | "منطقي" | "booléen" | "booleen" => Ok(()),
            other => Err(unsupported(&format!("the type `{}`", other))),
        },
        Some(_) => Err(unsupported("arrays or Result types")),
    }
}

struct FnGen<'a, 'b> {
    module: &'a mut JITModule,
    builder: FunctionBuilder<'b>,
    ids: &'a HashMap<String, (FuncId, usize, bool)>,
    vars: Vec<HashMap<String, Variable>>,
    print_int: FuncId,
    print_sp: FuncId,
    print_nl: FuncId,
}

impl FnGen<'_, '_> {
    fn new_var(&mut self, name: &str) -> Variable {
        let var = self.builder.declare_var(types::I64);
        self.vars.last_mut().unwrap().insert(name.to_string(), var);
        var
    }

    fn lookup(&self, name: &str) -> Option<Variable> {
        self.vars.iter().rev().find_map(|s| s.get(name).copied())
    }

    /// Generate a block; returns its tail value (the value of a final
    /// bare-expression statement), used for implicit returns and if-expressions.
    fn gen_block(&mut self, b: &Block) -> JResult<Option<Value>> {
        self.vars.push(HashMap::new());
        let mut tail = None;
        for s in &b.stmts {
            tail = self.gen_stmt(s)?;
        }
        self.vars.pop();
        Ok(tail)
    }

    fn gen_stmt(&mut self, s: &Stmt) -> JResult<Option<Value>> {
        match s {
            Stmt::Let { name, init, .. } => {
                let v = self.gen_expr(init)?;
                let var = self.new_var(name);
                self.builder.def_var(var, v);
                Ok(None)
            }
            Stmt::Assign { target, value } => {
                let v = self.gen_expr(value)?;
                match target {
                    AssignTarget::Var(name) => {
                        let var = self
                            .lookup(name)
                            .ok_or_else(|| format!("unknown variable `{}`", name))?;
                        self.builder.def_var(var, v);
                        Ok(None)
                    }
                    _ => Err(unsupported("indexed or field assignment")),
                }
            }
            Stmt::While { cond, body } => {
                let header = self.builder.create_block();
                let body_b = self.builder.create_block();
                let exit = self.builder.create_block();
                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(header);
                let c = self.gen_expr(cond)?;
                self.builder.ins().brif(c, body_b, &[], exit, &[]);
                self.builder.switch_to_block(body_b);
                self.builder.seal_block(body_b);
                self.gen_block(body)?;
                self.builder.ins().jump(header, &[]);
                self.builder.seal_block(header);
                self.builder.switch_to_block(exit);
                self.builder.seal_block(exit);
                Ok(None)
            }
            Stmt::For {
                var,
                start,
                end,
                body,
            } => {
                let s = self.gen_expr(start)?;
                let e = self.gen_expr(end)?;
                self.vars.push(HashMap::new());
                let ivar = self.new_var(var);
                self.builder.def_var(ivar, s);

                let header = self.builder.create_block();
                let body_b = self.builder.create_block();
                let exit = self.builder.create_block();
                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(header);
                let i = self.builder.use_var(ivar);
                let c = self.builder.ins().icmp(
                    cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
                    i,
                    e,
                );
                self.builder.ins().brif(c, body_b, &[], exit, &[]);
                self.builder.switch_to_block(body_b);
                self.builder.seal_block(body_b);
                self.gen_block(body)?;
                let i = self.builder.use_var(ivar);
                let one = self.builder.ins().iconst(types::I64, 1);
                let next = self.builder.ins().iadd(i, one);
                self.builder.def_var(ivar, next);
                self.builder.ins().jump(header, &[]);
                self.builder.seal_block(header);
                self.builder.switch_to_block(exit);
                self.builder.seal_block(exit);
                self.vars.pop();
                Ok(None)
            }
            Stmt::ForEach { .. } => Err(unsupported("array iteration")),
            Stmt::Return(opt) => {
                match opt {
                    Some(e) => {
                        let v = self.gen_expr(e)?;
                        self.builder.ins().return_(&[v]);
                    }
                    None => {
                        self.builder.ins().return_(&[]);
                    }
                }
                // Anything after an explicit return is unreachable; park it
                // in a fresh sealed block.
                let dead = self.builder.create_block();
                self.builder.switch_to_block(dead);
                self.builder.seal_block(dead);
                Ok(None)
            }
            Stmt::Expr(e) => Ok(Some(self.gen_expr(e)?)),
        }
    }

    fn gen_expr(&mut self, e: &Expr) -> JResult<Value> {
        match &e.kind {
            ExprKind::Int(n) => Ok(self.builder.ins().iconst(types::I64, *n)),
            ExprKind::Bool(b) => Ok(self.builder.ins().iconst(types::I64, *b as i64)),
            ExprKind::Ident(name) => {
                let var = self
                    .lookup(name)
                    .ok_or_else(|| format!("unknown variable `{}`", name))?;
                Ok(self.builder.use_var(var))
            }
            ExprKind::Unary { op, rhs } => {
                let v = self.gen_expr(rhs)?;
                match op {
                    UnOp::Neg => Ok(self.builder.ins().ineg(v)),
                    UnOp::Not => {
                        let one = self.builder.ins().iconst(types::I64, 1);
                        Ok(self.builder.ins().bxor(v, one))
                    }
                }
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.gen_expr(lhs)?;
                let r = self.gen_expr(rhs)?;
                use cranelift_codegen::ir::condcodes::IntCC;
                let ins = self.builder.ins();
                let v = match op {
                    BinOp::Add => ins.iadd(l, r),
                    BinOp::Sub => ins.isub(l, r),
                    BinOp::Mul => ins.imul(l, r),
                    BinOp::Div => ins.sdiv(l, r),
                    BinOp::Rem => ins.srem(l, r),
                    BinOp::Eq => self.cmp(IntCC::Equal, l, r),
                    BinOp::Ne => self.cmp(IntCC::NotEqual, l, r),
                    BinOp::Lt => self.cmp(IntCC::SignedLessThan, l, r),
                    BinOp::Le => self.cmp(IntCC::SignedLessThanOrEqual, l, r),
                    BinOp::Gt => self.cmp(IntCC::SignedGreaterThan, l, r),
                    BinOp::Ge => self.cmp(IntCC::SignedGreaterThanOrEqual, l, r),
                };
                Ok(v)
            }
            ExprKind::Call { callee, args } => {
                if is_print_builtin(callee) {
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            let f = self.func_ref(self.print_sp);
                            self.builder.ins().call(f, &[]);
                        }
                        let v = self.gen_expr(a)?;
                        let f = self.func_ref(self.print_int);
                        self.builder.ins().call(f, &[v]);
                    }
                    let f = self.func_ref(self.print_nl);
                    self.builder.ins().call(f, &[]);
                    // print evaluates to unit; give the caller a zero.
                    return Ok(self.builder.ins().iconst(types::I64, 0));
                }
                let (id, arity, has_ret) = *self
                    .ids
                    .get(callee)
                    .ok_or_else(|| unsupported(&format!("the callee `{}`", callee)))?;
                if args.len() != arity {
                    return Err(format!("function `{}`: wrong arity", callee));
                }
                let mut vals = Vec::new();
                for a in args {
                    vals.push(self.gen_expr(a)?);
                }
                let f = self.func_ref(id);
                let call = self.builder.ins().call(f, &vals);
                if has_ret {
                    Ok(self.builder.inst_results(call)[0])
                } else {
                    Ok(self.builder.ins().iconst(types::I64, 0))
                }
            }
            ExprKind::If { cond, then_b, els } => {
                let c = self.gen_expr(cond)?;
                let then_block = self.builder.create_block();
                let else_block = self.builder.create_block();
                let merge = self.builder.create_block();
                self.builder.append_block_param(merge, types::I64);

                self.builder.ins().brif(c, then_block, &[], else_block, &[]);

                self.builder.switch_to_block(then_block);
                self.builder.seal_block(then_block);
                let tv = self.gen_block(then_b)?;
                let tv = tv.unwrap_or_else(|| self.builder.ins().iconst(types::I64, 0));
                self.builder.ins().jump(merge, &[tv.into()]);

                self.builder.switch_to_block(else_block);
                self.builder.seal_block(else_block);
                let ev = match els {
                    Some(e) => self.gen_expr(e)?,
                    None => self.builder.ins().iconst(types::I64, 0),
                };
                self.builder.ins().jump(merge, &[ev.into()]);

                self.builder.switch_to_block(merge);
                self.builder.seal_block(merge);
                Ok(self.builder.block_params(merge)[0])
            }
            ExprKind::Block(b) => {
                let tail = self.gen_block(b)?;
                Ok(tail.unwrap_or_else(|| self.builder.ins().iconst(types::I64, 0)))
            }
            ExprKind::Str(_) | ExprKind::StrInterp(_) => Err(unsupported("strings")),
            ExprKind::Float(_) => Err(unsupported("floats")),
            ExprKind::Cast { .. } => Err(unsupported("casts")),
            ExprKind::ArrayLit(_) | ExprKind::Index { .. } => Err(unsupported("arrays")),
            ExprKind::Try(_) => Err(unsupported("`?` / Result")),
            ExprKind::StructLit { .. }
            | ExprKind::Field { .. }
            | ExprKind::Path { .. }
            | ExprKind::MethodCall { .. }
            | ExprKind::Match { .. } => Err(unsupported("structs, enums, or match")),
        }
    }

    fn cmp(&mut self, cc: cranelift_codegen::ir::condcodes::IntCC, l: Value, r: Value) -> Value {
        let b = self.builder.ins().icmp(cc, l, r);
        // Comparisons produce I8; widen to our universal I64.
        self.builder.ins().uextend(types::I64, b)
    }

    fn func_ref(&mut self, id: FuncId) -> cranelift_codegen::ir::FuncRef {
        self.module.declare_func_in_func(id, self.builder.func)
    }
}
