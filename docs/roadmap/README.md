# TN619 Roadmap — Index

One file per design phase. Each phase file lists **what's done**, **what's not**,
and the **forward plan** for that phase.

Status legend: ✅ done · 🟡 partial / in progress · ⬜ not started

| # | Phase | Status | File |
|---|-------|--------|------|
| 1 | Language Vision | ✅ | [phase-01-vision.md](phase-01-vision.md) |
| 2 | Language Design (syntax) | ✅ | [phase-02-language-design.md](phase-02-language-design.md) |
| 3 | Bilingual System | 🟡 | [phase-03-bilingual.md](phase-03-bilingual.md) |
| 4 | Compiler Architecture | ✅ (design) | [phase-04-compiler-architecture.md](phase-04-compiler-architecture.md) |
| 5 | Memory Model | ✅ (design) | [phase-05-memory-model.md](phase-05-memory-model.md) |
| 6 | Ecosystem & Tooling | ⬜ | [phase-06-ecosystem.md](phase-06-ecosystem.md) |
| 7 | Standard Library | ⬜ | [phase-07-stdlib.md](phase-07-stdlib.md) |
| 8 | Roadmap | ✅ | [phase-08-roadmap.md](phase-08-roadmap.md) |
| 9 | Project Structure | 🟡 | [phase-09-project-structure.md](phase-09-project-structure.md) |
| 10 | First MVP | ✅ (shipped, superseded by M1–M3) | [phase-10-mvp.md](phase-10-mvp.md) |
| — | **Next phases (M4+)** | ⬜ forward plan | [NEXT-PHASES.md](NEXT-PHASES.md) |

## Overall snapshot (2026-06-10)

- **Design:** Phases 1, 2, 4, 5 complete; 3 folded into the lexer (+ hardening
  underway); 8 done.
- **Built & running:** a **trilingual** (en/ar/fr) compiler — lexer → parser →
  static type checker → tree-walking interpreter, plus a **Cranelift JIT**
  (`tnc jit`) for the int/bool subset. The checker enforces `match`
  exhaustiveness, compile-time mutability, and an **ownership move checker**.
  Language: arrays, `Result<T, E>` + `?`, `&mut self`, structs/enums/methods,
  interpolation, floats. 35 integration tests. See Phase 10 and
  [NEXT-PHASES.md](NEXT-PHASES.md).
- **Not started:** ecosystem/tooling (6), standard library (7) — scheduled as
  M8 and M7 in [NEXT-PHASES.md](NEXT-PHASES.md).
- **Biggest open build item:** generics/traits (M4), then the full borrow
  checker designed in Phase 5 (M5).

See also the consolidated [../ROADMAP.md](../ROADMAP.md) (24-month plan) and the
[memory-model design](../design/05-memory-model.md).
