# Phase 2 — Language Design (syntax) · ✅ Done (design)

**Goal:** specify full syntax (English + Arabic) for every feature, with internal
representation + Rust comparison.

## ✅ Done (designed)
- [x] Foundational choices: braces (RTL-neutral), optional semicolons (Go-style
      ASI), expression-oriented, postfix types, immutable by default, no classes.
- [x] Variables `let`/`var` (`دع`/`متغير`), constants, local type inference.
- [x] Functions (implicit tail return, `->` types), conditions (no truthiness),
      loops (`for`/`while`/`loop`).
- [x] Structs + `impl`, enums (ADTs), pattern matching (exhaustive), generics
      (monomorphized + planned polymorphization).
- [x] Traits (coherence kept + `wrapper` sugar), modules (automatic file-as-module).
- [x] Error handling (`Result` + `?`/`؟`, no exceptions), macros (one hygienic
      declarative system; `comptime` as future direction).

## 🟡 Built (subset implemented in the MVP — see Phase 10)
- [x] variables, arithmetic, functions, if/else, while, for-range, structs, enums,
      match, methods, type inference for locals.
- [ ] traits, generics, modules, error handling, macros — designed, not yet built.

## ⬜ Not done / open
- [ ] **Async / concurrency syntax** — deliberately deferred (deep-dive pending).
- [ ] String interpolation details (`"{x}"`) — designed informally, not formalized.
- [ ] Closures / lambdas — referenced in Phase 2 examples, need a full spec.
- [ ] Sized integer types (`i32`/`i64`/`u…`) — MVP uses one `int`; formalize later.

## Forward plan
1. Formalize closures + string interpolation (needed by stdlib).
2. Design async/concurrency as its own focused phase.
3. Finalize the numeric type tower (`int` → `i8..i64`, `u8..u64`, `f32`/`f64`).
