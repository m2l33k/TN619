# Phase 3 — Bilingual System · 🟡 Partial (core built + perf-optimized; hardening pending)

**Goal:** robust Arabic + English syntax over one backend, one AST, one semantics.

_Last updated: 2026-06-04 (lexer performance rewrite)._

## ✅ Done
- [x] One keyword map holding BOTH spellings → one language-neutral `TokenKind`
      (`token.rs`). Source language is **erased at the lexer boundary**.
- [x] Digit folding: Arabic-Indic (٠-٩) **and** Extended/Persian (۰-۹) → 0-9.
- [x] Bilingual punctuation: `،`→Comma, `؟`→Try-position, Arabic identifiers,
      mixed AR/EN source files.
- [x] Identifiers may contain digits after the first char (`count1`, `عداد٢`).
- [x] `self`/`الذات` and entry point `main`/`رئيسي` handled bilingually.
- [x] Proof: `دع age = ٢٠` lexes to the same token stream as `let age = 20`.
- [x] **Performance rewrite (zero-copy byte cursor):** the lexer walks the source
      `&str` by byte offset — no `Vec<char>` allocation, keywords matched on a
      borrowed `&str` slice (no allocation), integer literals accumulated directly
      into `i64`. Heap allocation only for `Ident`/`Str` payloads. Verified on a
      1.39 MB / 20,001-function mixed EN/AR file (full lex+parse+typecheck ~0.5s
      incl. process startup).
- [x] **Trojan-Source / bidi-control rejection (CVE-2021-42574):** the lexer
      rejects the 9 bidirectional text-flow control characters
      (U+202A-202E, U+2066-2069) anywhere in source, with a clear diagnostic.
      Dependency-free; uses the same codepoint set as rustc. (Adopted from the
      rustc_lexer study, 2026-06-04.)

## ⚡ Performance (lexer)
The bilingual lexer (`compiler/tnc/src/lexer.rs`) is a **zero-copy byte-cursor**
scanner. Hot-path costs removed vs the original:

| Aspect | Before | After |
|--------|--------|-------|
| Source storage | `Vec<char>` allocated for whole file | walk `&str` by byte offset (no alloc) |
| Keyword lookup | `String` built per word, then matched | match on borrowed `&str` slice (no alloc) |
| Integer literals | temp `String` + `.parse()` | accumulated directly into `i64` (checked) |
| Token buffer | grown on demand | pre-sized to `src.len()/4 + 8` |
| Heap allocation | every identifier/keyword | only `Ident` / `Str` payloads |

Measured: 1.39 MB / 20,001-function mixed EN/AR source → full lex+parse+typecheck
in ~0.5 s (including process startup). No behavior change; all examples identical.

## ⬜ Not done (security + correctness hardening)
- [ ] **Unicode XID identifiers** — adopt `XID_Start`/`XID_Continue` (via the
      `unicode_ident` crate, as rustc does) instead of `char::is_alphabetic`/
      `is_alphanumeric`. The spec-correct identifier rule for Arabic + global text.
- [ ] **Unicode NFC normalization** of identifiers (needs `unicode-normalization`).
      Without it, visually-identical identifiers can differ byte-wise.
- [ ] **Confusable detection** — a table mapping look-alike Unicode chars to ASCII
      (rustc's `unicode_chars.rs` `UNICODE_ARRAY`) to suggest fixes / warn.
- [ ] Invisible-character handling (zero-width joiners etc.) beyond the bidi set.
- [ ] Graceful error recovery for emoji/invalid identifiers (rustc's `InvalidIdent`).
- [ ] RTL editor/formatter guidance + canonical per-file language rules.
- [ ] Full bilingual keyword coverage as new keywords are added (traits, async…).

## Forward plan
Mapped to Phase 8 milestone **M14 (Q7)**:
1. Add NFC normalization in the lexer (fold before keyword lookup + interning).
2. Reject bidi control characters by default; flag with a clear diagnostic.
3. Add a confusables lint.
4. Bilingual formatting rules in `tnfmt` (Phase 6).

## Changelog
- **2026-06-04** — Added **Trojan-Source / bidi-control rejection** (9 codepoints,
  dependency-free), informed by studying `rustc_lexer` / `rustc_ast::util::unicode`.
  Recorded XID identifiers + confusable detection as next steps.
- **2026-06-04** — Lexer rewritten as a zero-copy byte-cursor scanner (see
  Performance). Added Extended/Persian digit folding (۰-۹). Fixed identifiers so
  they may contain digits after the first character (`count1`, `عداد٢`).
- **(earlier)** — Initial bilingual lexer: shared keyword map, Arabic-Indic digit
  folding, bilingual comma, mixed-script source.

## Reference notes (from studying rust-lang/rust)
- `rustc_lexer` is a standalone crate emitting raw tokens (no spans/interning) —
  matches our planned `tn_lexer` split. Cursor over chars with `first/second/third`
  lookahead; identifiers use `unicode_ident` (XID). Emoji lex to `InvalidIdent`
  for recovery.
- Confusables: `rustc_parse/src/lexer/unicode_chars.rs` `UNICODE_ARRAY` maps
  look-alikes → ASCII with fix suggestions.
- Trojan Source: `TEXT_FLOW_CONTROL_CHARS` in `rustc_ast::util::unicode` =
  `U+202A,202B,202C,202D,202E,2066,2067,2068,2069`; rustc *lints* them in
  comments/literals — TN619 *rejects* them everywhere (stricter, secure-by-default).
