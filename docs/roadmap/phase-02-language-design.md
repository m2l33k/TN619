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

## ✅ Done — remaining feature specs (docs/design/02-language-features.md)
- [x] **Numeric type tower** — `i8..i64`/`u8..u64`/`isize`/`usize`/`f32`/`f64`
      with bilingual names (`صN`/`طN`/`عN`), inference + defaulting (`i64`/`f64`),
      checked overflow, explicit `as`/`كـ` casts.
- [x] **String interpolation** — `"{expr}"`, `{{`/`}}` escapes; lexing into
      literal/expr parts + desugaring to a `Display` builder.
- [x] **Closures / lambdas** — `|x| ...` syntax, capture/ownership model
      (borrow by default, `move`/`نقل`), single `Callable` trait (vs Rust's three).
- [x] **Async / concurrency** — design direction chosen: **structured concurrency
      (nurseries)** atop async, with rationale vs async-await and Go-style tasks.

## ✅ Resolved (was open)
- [x] **Async** surface syntax finalized (keywords `async/متزامن`, `await/انتظر`,
      `nursery/حضانة`, `spawn/أطلق`, `channel/قناة`); its *semantics* are
      deliberately scoped to a **dedicated future phase**, not Phase 2 (a Phase 1
      non-goal). So it no longer blocks Phase 2.
- [x] **String interpolation implemented in the MVP** end-to-end (lexer → parser →
      type checker → interpreter), bilingual — `examples/interp_en.tn` /
      `interp_ar.tn`, covered by tests.

## ⬜ Tracked under Phase 10 (implementation, not design)
- [ ] Numeric tower (f64 + sized ints) — implement in the MVP.
- [ ] Closures — implement after the type checker supports function types.
(These are *build* tasks; Phase 2 *design* is complete.)

## Forward plan
Phase 2 (design) is complete. Remaining work is implementation, tracked in
Phase 10; async semantics is a separate future phase.
