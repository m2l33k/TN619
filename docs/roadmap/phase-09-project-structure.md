# Phase 9 — Project Structure · 🟡 Partial

**Goal:** the full GitHub monorepo architecture.

## ✅ Done
- [x] Cargo workspace root (`Cargo.toml`).
- [x] `compiler/tnc/` single crate with internal modules
      (`token`, `lexer`, `ast`, `parser`, `typeck`, `interp`, `main`).
- [x] `examples/` with bilingual sample programs.
- [x] `docs/` with design + roadmap.
- [x] `README.md`.
- [x] `tests/` — integration suite (`compiler/tnc/tests/run_examples.rs`):
      golden-output tests for every bilingual example + rejection tests +
      Trojan-Source security regression. 11 tests, all green.
- [x] **CI** (`.github/workflows/ci.yml`): build + test + `cargo fmt --check` on
      every push/PR.
- [x] `LICENSE-MIT` (dual MIT OR Apache-2.0 intent), `.gitignore`.

## ⬜ Not done (target monorepo)
- [ ] Split `tnc` into per-stage crates: `tn_span`, `tn_errors`, `tn_lexer`,
      `tn_ast`, `tn_parser`, `tn_hir`, `tn_resolve`, `tn_types`, `tn_typeck`,
      `tn_mir`, `tn_borrowck`, `tn_codegen_llvm`, `tn_codegen_clif`, `tn_driver`,
      `tn_cli`.
- [ ] `std/` — standard library (Phase 7).
- [ ] `tnpkg/`, `tnfmt/`, `tnlint/`, `tnls/`, `tndoc/` — tooling (Phase 6).
- [ ] `LICENSE-APACHE` (the Apache-2.0 half of the dual license).
- [ ] `clippy` in CI; issue templates, CONTRIBUTING.
- [ ] Benchmarks harness.

## Forward plan
1. Add a `tests/` integration suite NOW (cheap, high value) — bilingual
   golden-output tests for every example.
2. Add CI early (GitHub Actions: build + test on push).
3. Do the **per-stage crate split** when MIR/codegen land (Q2–Q3), migrating the
   AST to arena/NodeId at the same time.
4. Add tooling crates as Phase 6 progresses; `std/` as Phase 7 progresses.

**Principle:** the folder layout should mirror the compiler's dependency graph
(each crate depends only on earlier stages).
