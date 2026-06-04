# Phase 4 — Compiler Architecture · ✅ Done (design); 🟡 partially built

**Goal:** full pipeline, implementation-language choice, LLVM integration.

## ✅ Done (designed)
- [x] Impl language: **Rust** (ADTs, safe compiler, `salsa`, `inkwell`).
- [x] Architecture: **query-based incremental (salsa)** — fast rebuilds + LSP.
- [x] Backends: **Cranelift (debug)** + **LLVM (release)**.
- [x] Full pipeline: lexer → parser → name resolution → HIR → type check → MIR →
      borrow check + lifetime inference → codegen → native (x86/ARM/RISCV/WASM).
- [x] AST design: arena + `NodeId` + interned `Symbol` + side tables.
- [x] Parser strategy: hand-written recursive descent + Pratt.

## 🟡 Built so far (MVP)
- [x] Lexer (hand-written, bilingual).
- [x] Parser (recursive descent + Pratt).
- [x] AST (simple tree; NOT yet arena/NodeId — MVP uses `Box`).
- [x] Static type checker (a pragmatic stand-in for the HIR+typeck stages).
- [x] Tree-walking interpreter (stands in for MIR + codegen).

## ⬜ Not done
- [ ] Name-resolution as a distinct pass (currently implicit in interp/typeck).
- [ ] HIR (desugaring layer) and MIR (CFG).
- [ ] salsa query architecture / incremental compilation.
- [ ] LLVM backend (inkwell) and Cranelift backend.
- [ ] Arena/NodeId AST + interned symbols + side tables.
- [ ] Per-stage crate split (see Phase 9).
- [ ] Spans on every node + first-class diagnostics infrastructure.

## Forward plan (mapped to Phase 8)
1. **M2 (Q1):** add spans + diagnostics infrastructure (before the codebase grows).
2. **M4 (Q2):** introduce MIR for the borrow checker.
3. **M5–M6 (Q3):** LLVM then Cranelift backends.
4. **M8 (Q4):** salsa incremental refactor.
5. Migrate AST to arena/NodeId during the crate split (Phase 9).
