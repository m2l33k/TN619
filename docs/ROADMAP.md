# TN619 — Roadmap

A modern, bilingual (English + Arabic) systems programming language: Rust-class
safety and performance, beginner-friendly, secure by design, LLVM-backed.

This file tracks **every design phase** (1–10) one by one, then the **24-month
month-by-month build plan** with milestones, risks, and blockers.

Status legend: ✅ done · 🟡 partial / in progress · ⬜ not started

---

## Part A — Phases, one by one

### Phase 1 — Language Vision ✅
**Goal:** define what TN619 is and why it should exist.
**Delivered:**
- Mission: *Rust-class safety & performance without Rust-class friction; the first
  systems language designed bilingually from the compiler up.*
- The moat (real differentiators): (1) **bilingual-native compiler** — AR/EN as
  first-class lexical front-ends over ONE AST; (2) **lifetime inference by
  default**. Everything else (safety, perf, fast compile, errors) is table stakes
  done well.
- Beachhead audience: Arabic-speaking students / new systems programmers (no
  native systems language exists for ~450M speakers); expand to Rust-fatigued
  engineers later.
- Principles: safety non-negotiable / ergonomics is the innovation; one semantics
  many surfaces; the error message is part of the language; compile speed is a
  budget; secure by default; no hidden cost; beginner-friendly ≠ dumbed-down.
- Explicitly **not** promising to fix Rust's async at launch.

### Phase 2 — Language Design ✅
**Goal:** specify the full syntax (English + Arabic) for every feature.
**Delivered (with internal representations + Rust comparisons):** variables
(`let`/`var` = `دع`/`متغير`), constants, type inference (local/bidirectional),
functions, conditions, loops (`for`/`while`/`loop`), structs + `impl`, enums
(ADTs), pattern matching (exhaustive), generics (monomorphized + planned
polymorphization), traits (coherence kept + `wrapper` sugar), modules
(**automatic file-as-module**), error handling (`Result` + `?`/`؟`, no
exceptions), macros (one hygienic declarative system; `comptime` as the future
direction, not proc-macros).
**Key choices:** braces not indentation (RTL-neutral), optional semicolons,
expression-oriented, no classes/inheritance, no truthiness, immutable by default.

### Phase 3 — Bilingual System ✅ (folded into the lexer)
**Goal:** robust AR + EN architecture over one backend.
**Delivered (designed + partly built):** one keyword map holding both spellings →
one language-neutral `TokenKind`; the source language is **erased at the lexer
boundary**. Plus the three hard problems: **Unicode NFC normalization**,
**Arabic-Indic digit folding** (٠-٩ → 0-9), and **bidi-control rejection**
(Trojan-Source / CVE-2021-42574 defense). Digit folding + bilingual comma/`؟`
are already implemented; NFC + bidi rejection are documented hooks pending the
relevant crates.

### Phase 4 — Compiler Architecture ✅
**Goal:** full pipeline, implementation choices, LLVM integration.
**Delivered:**
- Impl language: **Rust** (ADTs for AST, safe compiler, `salsa`, `inkwell`).
- Architecture: **query-based incremental (salsa)** — fast rebuilds + near-free LSP.
- Backends: **Cranelift (debug, fast compile)** + **LLVM (release, fast code)**.
- Pipeline: lexer → parser → name resolution → HIR (desugar) → type check → MIR →
  borrow check + lifetime inference → codegen → native (x86/ARM/RISCV/WASM).
- AST: arena + `NodeId` + interned `Symbol` + side tables (salsa-friendly).
- Parser: hand-written recursive descent + Pratt (for first-class diagnostics).

### Phase 5 — Memory Model ✅ (design)
**Goal:** ownership safer than C++, easier than Rust, zero-cost, no GC.
**Delivered:** [docs/design/05-memory-model.md](design/05-memory-model.md).
Single ownership + moves, shared-XOR-mutable borrows, two-layer immutability,
**lifetime inference** as region constraint solving, inferred `Copy` /
`Sendable` / `Shareable`, auto-borrow, deterministic drop.
**Locked decision:** *private-infer / public-explicit-when-ambiguous* lifetimes.
**Implementation rule:** sound-first — conservative is OK, unsound never.

### Phase 6 — Ecosystem & Tooling ⬜
**Goal:** the developer-experience layer. **Planned names:** `tnpkg` (package
manager), `tn build` (build system), `tnfmt` (formatter), `tnlint` (linter),
`tndoc` (doc generator), `tnls` (LSP server), debugger via DWARF + LLDB/GDB.
Bilingual formatting (canonical per-file language, optional transliteration view)
is the novel challenge here.

### Phase 7 — Standard Library ⬜
**Goal:** stdlib modules — IO, collections, fs, networking, concurrency, crypto,
JSON, http/web, database, logging — with **security-first** defaults (constant-time
crypto primitives, tainted-input tracking in the type system, safe-by-default
parsers). Bilingual API naming strategy to be decided.

### Phase 8 — Roadmap ✅ (this file, Part B)

### Phase 9 — Project Structure 🟡
**Goal:** full monorepo. **Current:** Cargo workspace with a single `tnc` crate
(MVP). **Planned:** split into per-stage crates (`tn_lexer`, `tn_parser`,
`tn_typeck`, `tn_borrowck`, `tn_codegen_*`, …) + `std/`, `tnpkg/`, `tnls/`,
`docs/`, `examples/`, `tests/`. See README for the current layout.

### Phase 10 — First MVP ✅ → 🟡 (growing)
**Goal:** minimal working compiler. **Delivered (running, EN+AR):** variables,
checked arithmetic, functions, `print`, `if`/`else` (expression), `while`,
`for..in a..b`, recursion, structs, enums, exhaustive `match`, field access,
methods + associated functions, a **static type checker**, and a security-
hardened bilingual lexer (Persian digits, Trojan-Source rejection). Backend is a
tree-walking interpreter (LLVM/Cranelift swap in later). ~2,470 LOC, zero deps.

---

## Part B — 24-Month Build Plan

Grounded in the actual experience of building M0–M0.3. Each milestone lists its
focus, exit criteria, and the main risk/blocker.

### Quarter 1 (Months 1–3): Solidify the front-end
- **M1 (M1–2): Language completeness in the interpreter.** Add references &
  `&mut self`, arrays/slices, a `Vec`, `f64`, string ops, and `while let`.
  *Exit:* can write non-trivial bilingual programs. *Risk:* `&mut` needs a real
  reference value in the interpreter (the value-clone model breaks here).
- **M2 (M3): Diagnostics overhaul.** Spans on every node, tiered bilingual error
  messages, did-you-mean (Levenshtein). *Exit:* errors point at exact source with
  fixes. *Risk:* retrofitting spans onto the AST — do it before the codebase grows.

### Quarter 2 (Months 4–6): Real type system + ownership
- **M3 (M4–5): Generics + traits in the checker.** Monomorphization-ready type
  checking, trait resolution, bounds. *Risk:* trait coherence + inference
  interaction is the subtlest part of the whole compiler.
- **M4 (M6): Move + borrow checker (Phase 5, steps 1–2).** Build MIR; implement
  ownership/move checking, then shared-XOR-mutable. *Exit:* use-after-move and
  aliasing violations are compile errors. *Risk:* this is the hardest subsystem;
  start conservative (more explicit `&`), keep it SOUND.

### Quarter 3 (Months 7–9): Native codegen
- **M5 (M7–8): LLVM backend (release).** MIR → LLVM IR via inkwell; native
  executables; checked arithmetic via overflow intrinsics. *Exit:* compiled
  programs match interpreter output and run natively. *Risk:* LLVM/inkwell setup
  + ABI details; cross-platform later.
- **M6 (M9): Cranelift backend (debug) + dual-mode driver.** *Exit:* `tn build`
  uses Cranelift (fast), `tn build --release` uses LLVM. *Risk:* maintaining two
  backends from one MIR.

### Quarter 4 (Months 10–12): Lifetime inference + incremental core
- **M7 (M10–11): Lifetime inference (Phase 5, steps 3–4).** Region solver over
  MIR; local inference, then `pub` single-input inference; explicit only for
  ambiguous public APIs. *Exit:* typical programs have zero `'a`. *Risk:* THE
  bet — validate on a large good/bad test corpus; soundness over ergonomics.
- **M8 (M12): salsa query refactor.** Make the pipeline incremental. *Exit:* edit
  one function → only dependent queries recompute. *Risk:* big architectural
  change; the earlier it lands, the cheaper.

### Quarter 5 (Months 13–15): Tooling (Phase 6)
- **M9 (M13): `tnpkg` + `tn build`.** Manifest format, dependency resolution,
  build orchestration mapping the filesystem to the module graph.
- **M10 (M14): `tnfmt` + `tnlint`.** Canonical formatting (incl. bilingual rules)
  and lints (naming, `panic`-in-lib, glob-import warnings).
- **M11 (M15): `tnls` (LSP) — built on the salsa queries.** Hover, go-to-def,
  diagnostics, completion. *Risk:* low if M8 done right (LSP reuses the query DB).

### Quarter 6 (Months 16–18): Standard library (Phase 7)
- **M12 (M16): core + collections + IO + fs.** *Risk:* stdlib API churn; freeze
  a `core` subset early.
- **M13 (M17–18): networking, crypto, JSON, logging — security-first.**
  Constant-time primitives; tainted-input type tracking. *Risk:* crypto must be
  correct — lean on vetted algorithms, audit early.

### Quarter 7 (Months 19–21): Hardening + bilingual polish
- **M14 (M19): NFC normalization + Trojan-Source/bidi rejection in the lexer.**
  Pull in `unicode-normalization` + `unicode-bidi`; identifier security tests.
- **M15 (M20–21): performance + robustness.** Benchmark vs Rust/C++; fuzz the
  parser/checker; large real-world test programs in both languages.

### Quarter 8 (Months 22–24): Self-hosting push + 1.0 prep
- **M16 (M22–23): begin self-hosting.** Rewrite the lexer + parser in TN619,
  compiled by the Rust bootstrap. *Risk:* the language must be expressive enough
  (depends on M1–M13); partial self-hosting is a fine 24-month target.
- **M17 (M24): `0.1` public release.** Docs (`tndoc`), tutorials (bilingual),
  playground, example corpus, contribution guide. *Exit:* outsiders can install
  `tnpkg`, write, build, and run bilingual TN619 programs.

---

## Cross-cutting risks (watch the whole way)
1. **Borrow checker soundness** (M4, M7) — the existential technical risk. Sound-first, always.
2. **Lifetime-inference ergonomics vs stability** (M7) — the public-boundary policy must hold up on real code.
3. **Scope creep from bilingualism** — every feature doubles its surface tests (EN + AR). Budget for it.
4. **Compile-time cost of monomorphization** (M3, M5) — invest in polymorphization early.
5. **Solo/small-team bandwidth** — the 24-month plan assumes focus; cut stdlib breadth before cutting safety.

## Definition of "done" for 0.1
Bilingual source · ownership + borrow checking + inferred lifetimes · native LLVM
codegen · `tnpkg`/`tnfmt`/`tnls` · a usable stdlib core · tiered bilingual
diagnostics · partial self-hosting. Safety is never traded for any deadline.
