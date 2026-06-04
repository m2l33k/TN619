# Phase 1 — Language Vision · ✅ Done

**Goal:** define what TN619 is, who it's for, and why it should exist.

## ✅ Done
- [x] Mission statement: *Rust-class safety & performance without Rust-class
      friction; the first systems language designed bilingually from the
      compiler up.*
- [x] Differentiators / the moat: (1) bilingual-native compiler (AR/EN over ONE
      AST, not a transpiler); (2) lifetime inference by default.
- [x] Beachhead audience: Arabic-speaking students / new systems programmers;
      expand to Rust-fatigued engineers later.
- [x] Core principles: safety non-negotiable; one semantics, many surfaces; the
      error message is part of the language; compile speed is a budget; secure by
      default; no hidden cost; beginner-friendly ≠ dumbed-down.
- [x] Manifesto (6 tenets) + explicit non-goal: not fixing Rust's async at launch.

- [x] **License chosen & added:** dual **MIT OR Apache-2.0** (`LICENSE-MIT`,
      `LICENSE-APACHE`) — the Rust-ecosystem convention; Apache adds a patent grant.
- [x] **Positioning written:** `docs/design/positioning.md` — brand/tagline
      ("Learn systems programming in your language"), competitive table vs
      Rust/Go/Swift/Zig/C++, target segments, and honest non-goals.

## ⬜ Not done / open
- [ ] Logo (wordmark readable LTR + RTL).
- [ ] Domain registration (`tn619.dev` / `.org`), GitHub org, `tnpkg` registry name.

## Forward plan
Vision + positioning + license are done. Remaining items are external/branding
(logo, domain) — tie to a public-release push (Phase 8 milestone M17). Revisit the
vision only if the beachhead or the core bet changes.
