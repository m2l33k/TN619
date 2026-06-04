# Phase 10 — First MVP · 🟡 Working & growing

**Goal:** a minimal working bilingual compiler. **Backend:** tree-walking
interpreter (LLVM/Cranelift swap in later). ~2,470 LOC across 7 modules
(`token`, `lexer`, `ast`, `parser`, `typeck`, `interp`, `main`), zero external deps.

## ✅ Done (running, English + Arabic)
- [x] Lexer: bilingual keywords, Arabic-Indic **+ Persian** digit folding,
      digit-bearing identifiers, comments, optional `;`; **zero-copy byte cursor**;
      **Trojan-Source / bidi-control rejection** (security).
- [x] Parser: recursive descent + Pratt; struct-literal/block disambiguation.
- [x] Variables `let`/`var`, local type inference.
- [x] Arithmetic with **checked overflow** (overflow is an error, not wraparound).
- [x] Functions, recursion, implicit tail return.
- [x] `if`/`else` as an expression; conditions must be `bool` (no truthiness).
- [x] `while`, `for x in a..b`.
- [x] Structs, field access, struct literals (+ field shorthand).
- [x] Enums (unit + tuple variants).
- [x] Pattern matching `match` (literal/binding/wildcard/variant patterns).
- [x] **Methods & associated functions** (`impl`, `self`/`&self`).
- [x] **Static type checker** + **compile-time match exhaustiveness**.
- [x] **String interpolation** — `"{expr}"`, `{{`/`}}` escapes (bilingual);
      lexer → parser (re-parses each piece) → type checker → interpreter.
- [x] CLI: `tnc run|check|tokens`.
- [x] Examples: adult_en/ar, shapes_en/ar, points_en/ar, mixed, bad_type,
      bad_exhaustive, **polyglot** (English + Arabic combined in one program —
      mixed keywords across declarations, within one function, and even within a
      single expression), and **interp_en/ar** (string interpolation).

## ⬜ Not done (next MVP growth — see milestone M1)
- [ ] References + `&mut self` (needs a real reference value; blocks mutation
      through methods).
- [ ] Arrays / slices / `Vec`; indexing.
- [ ] `f64` and the full numeric type tower; sized ints.
- [ ] More string ops (concatenation, methods). _(Interpolation is done.)_
- [ ] `while let`, `loop` with `break value`, labeled loops.
- [ ] Closures / lambdas.
- [ ] Generics, traits, modules, error handling (`?`), macros (designed in Phase 2).
- [x] **Line numbers in diagnostics** — every type error and runtime error
      reports `line N: …` (expressions carry their source line). _(Column
      numbers + caret rendering + tiered/bilingual phrasing still to come.)_

## Forward plan (mapped to Phase 8, Q1)
1. **M1:** references + `&mut self`, arrays/`Vec`, `f64`, string ops, `while let`.
2. **M2:** spans + diagnostics overhaul (do before the codebase grows).
3. Then generics/traits (M3) and the borrow checker (M4) move the MVP toward a
   real compiler.

## How to run
```sh
cargo run -- run examples/points_ar.tn
cargo run -- check examples/bad_exhaustive.tn
```
