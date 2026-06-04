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
| 10 | First MVP | 🟡 (working, growing) | [phase-10-mvp.md](phase-10-mvp.md) |

## Overall snapshot (2026-06-04)

- **Design:** Phases 1, 2, 4, 5 complete; 3 folded into the lexer (+ hardening
  underway); 8 done.
- **Built & running:** a bilingual compiler — lexer → parser → static type checker
  → tree-walking interpreter (~2,470 LOC, 7 modules, zero deps), English + Arabic.
  Includes checked arithmetic, compile-time `match` exhaustiveness, and
  Trojan-Source (bidi-control) rejection. See Phase 10.
- **Not started:** ecosystem/tooling (6), standard library (7).
- **Biggest open build item:** the borrow checker designed in Phase 5.

See also the consolidated [../ROADMAP.md](../ROADMAP.md) (24-month plan) and the
[memory-model design](../design/05-memory-model.md).
