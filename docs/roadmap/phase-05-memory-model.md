# Phase 5 — Memory Model · ✅ Done (design); ⬜ not built

**Goal:** ownership safer than C++, easier than Rust; zero-cost, no GC.
**Full spec:** [../design/05-memory-model.md](../design/05-memory-model.md).

## ✅ Done (designed)
- [x] Ownership: single owner, move semantics, scope-based drop (RAII, no GC).
- [x] `Copy` vs move (inferred, not derived).
- [x] Borrows: `&T` shared XOR `&mut T` exclusive (compile-time data-race freedom).
- [x] Two-layer immutability: binding (`let`/`var`) + reference (`&`/`&mut`).
- [x] Lifetimes as **regions** solved by constraints; **inference** is the headline.
- [x] **Locked policy:** private-infer / public-explicit-when-ambiguous.
- [x] Auto-borrow / auto-deref; inferred `Sendable`/`Shareable` (Send/Sync).
- [x] Drop elaboration; sound-first implementation plan.

## ⬜ Not done (all implementation)
- [ ] MIR construction (prerequisite).
- [ ] Move / use-after-move checking (impl step 1).
- [ ] Borrow checking: shared-XOR-mutable via dataflow (step 2).
- [ ] Region/lifetime inference: local first, then `pub` single-input (steps 3–4).
- [ ] Drop elaboration; `Copy`/`Sendable`/`Shareable` inference (step 5).
- [ ] `&mut self` (blocked on a real reference value — the MVP clones values).

## Forward plan (mapped to Phase 8)
1. **M1 (Q1):** real references + `&mut self` in the interpreter (semantics first).
2. **M4 (Q2):** MIR + move checking + borrow checking (conservative, SOUND).
3. **M7 (Q4):** lifetime inference; validate on a large good/bad test corpus.
4. Drop + marker-trait inference alongside codegen (Q3) and stdlib (Q6).

**Golden rule:** conservative-but-sound is shippable; unsound never is.
