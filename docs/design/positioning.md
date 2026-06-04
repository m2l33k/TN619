# TN619 — Positioning, Brand & License (Phase 1 completion)

## Name & brand
- **Name:** TN619 (kept). Pronounced "T-N six-one-nine."
- **Tagline (primary):** *"Learn systems programming in your language."*
- **Tagline (technical):** *"Rust-class safety, bilingual by design."*
- **One-liner:** TN619 is a memory-safe, high-performance systems programming
  language whose source can be written natively in **Arabic or English** (or both),
  compiling to one identical program.
- **Brand voice:** approachable, honest, education-first. Not "Rust killer" —
  "the safe, fast language you can actually learn in your native language, that
  prepares you for the global ecosystem."

### Action items (external, not code)
- [ ] Logo (wordmark that reads cleanly LTR and RTL).
- [ ] Domain (candidates: `tn619.dev`, `tn619.org`, `tn619.lang`).
- [ ] Reserve names: `tnpkg` registry, GitHub org `tn619`.

## License — **dual MIT OR Apache-2.0**
TN619 is dual-licensed under **MIT** (`LICENSE-MIT`) **OR** **Apache-2.0**
(`LICENSE-APACHE`), at the user's option — the same convention as Rust and most
of its ecosystem.

**Why dual:**
- **Apache-2.0** provides an explicit **patent grant** (important for a compiler
  that may touch patented techniques) and a contributor license norm.
- **MIT** is maximally permissive and familiar, easing adoption and reuse.
- Offering both lets downstream users pick whichever fits their project, removing
  a friction point for contributors and companies.

## Positioning vs other languages
The honest competitive picture. TN619's moat is **native-language onboarding +
memory safety**, not raw novelty on any single axis.

| | Safety | Perf | Learning curve | GC | Native non-English | Niche |
|---|---|---|---|---|---|---|
| **C / C++** | unsafe | top | hard | no | no | legacy/maximal control |
| **Rust** | safe | top | **steep** | no | no | safety + perf, expert tier |
| **Go** | safe (GC) | good | easy | **yes** | no | services/networking |
| **Swift** | safe (ARC) | good | medium | ARC | no | Apple ecosystem |
| **Zig** | manual-safe | top | medium | no | no | C replacement, comptime |
| **TN619** | safe (no GC) | target ≈ Rust | **easier than Rust** | no | **yes (AR+EN)** | native-language systems education |

### How we talk about each
- **vs Rust:** "Rust's safety and performance, with the lifetime annotations
  inferred and the keywords in your language. The on-ramp Rust never had."
- **vs Go:** "Go is easy but garbage-collected and English-only. TN619 is easy,
  *and* zero-cost/no-GC, *and* speaks your language."
- **vs Swift:** "Swift is friendly but tied to Apple and uses ARC. TN619 is
  cross-platform with no reference-counting overhead."
- **vs Zig:** "Zig trusts you with manual memory; TN619 gives you compile-time
  memory safety. Both value simplicity; we add safety and bilingualism."
- **vs C/C++:** "The performance, without the footguns — and learnable in Arabic."

## Target segments (priority order)
1. **Arabic-speaking CS students & self-learners** (beachhead).
2. **Educators / universities / bootcamps** in MENA.
3. **The localization thesis:** the same architecture localizes to *any* language
   (Hindi, Spanish, Swahili, …) — each an underserved market.
4. **Pragmatic systems engineers** wanting Rust-grade safety with less friction
   (growth tier, later).

## The honest non-goals
- Not trying to displace English as the global default; English interop is always
  first-class so learners can "graduate to the world."
- Not a Rust replacement for production systems on day one.
- Free-mixing EN/AR is a *capability*, not the recommended team style — `tnfmt`
  enforces per-file consistency and can transliterate between languages.
