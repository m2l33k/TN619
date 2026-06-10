# TN619 — Next Phases (M4+)

Forward plan picking up after the M0–M3 milestones, all of which are **built,
tested, and passing** (35 integration tests as of 2026-06-10):

- **M0** — trilingual MVP: lexer → parser → type checker → interpreter;
  English + Arabic + French surfaces over one neutral AST; structs, enums,
  match + exhaustiveness, methods, interpolation, floats.
- **M1** — usability: arrays `[T]` (len/push/pop, bounds-checked),
  `Result<T, E>` + `?` (incl. `؟`), `&mut self` with write-back,
  compile-time mutability, `a[i] =` / `x.f =`, `for x in arr`.
- **M2** — ownership: compile-time move checker (use-after-move, loop moves,
  branch-aware merging, borrowed-`self` protection) with
  `.clone()`/`انسخ()`/`.cloner()` as the explicit copy.
- **M3** — native code: `tnc jit` compiles the int/bool subset to machine
  code via Cranelift; output verified identical to the interpreter.

Status legend: ✅ done · 🟡 partial · ⬜ not started

---

## M4 — Language completeness ⬜

The features that make real programs pleasant, and that remove the last
"special cases" from the compiler.

| Work item | Why | Notes |
|---|---|---|
| **Generics** (`fn max<T>(..)`, `struct Pair<T>`) | `[T]` and `Result<T, E>` are currently compiler special cases; user code deserves the same power | Monomorphization at check time fits the current architecture |
| **Traits** (interfaces) | Needed for `==`/ordering on user types, display, iteration | Start with built-in traits (`Eq`, `Ord`, `Show`), then user traits |
| **Closures** (`|x| x + 1`) | Higher-order stdlib (`map`, `filter`) is impossible without them | Capture-by-move fits the ownership model already built |
| **Modules & imports** | Programs are single files today | One file = one module; `use`/`استورد`/`importe` |
| **String methods** | `str` has no API beyond `+`-free interpolation | `split`, `contains`, `to_int` — trilingual names, same table mechanism as array methods |

**Exit criteria:** a generic `fn map<T, U>(xs: [T], f: |T| -> U) -> [U]` written
in TN619 type-checks and runs; `Vec`/`Result` lose their special-cased payload
typing internally.

## M5 — References & full borrow checker ⬜

The designed-but-unbuilt heart of the memory model
([../design/05-memory-model.md](../design/05-memory-model.md)).

- General `&T` / `&mut T` parameters and locals (today only `self` can be borrowed).
- Lifetime **inference** (no explicit lifetime syntax — a TN619 design goal).
- Replace the M2 simplifications: field projections currently *implicitly
  clone*; with real borrows, `x.f` reads become borrows and partial moves get
  tracked per-field.
- Borrow rules: one `&mut` xor many `&`, checked at compile time.

**Exit criteria:** the M2 move checker upgrades from "whole-variable moves"
to true places (fields, indices); passing `&xs` to a function no longer
requires either a move or a clone.

## M6 — Native backend growth 🟡 (subset exists)

Grow `tnc jit` from the int/bool subset toward the full language, keeping the
interpreter as the differential-testing oracle (the `jit_matches_interpreter`
test is the pattern: same program, both backends, identical output).

1. Integer overflow **traps** (parity with the interpreter's checked arithmetic).
2. Strings (static data + a small runtime), then structs (stack slots),
   then arrays (heap + bounds-check traps), then enums/match (tag + jump table).
3. `tnc build` — AOT: emit an object file + link to a real executable
   (cranelift-object), not just in-process JIT.
4. Backend boundary: keep codegen behind one trait so an LLVM release backend
   can slot in later (Phase 4 architecture doc).

**Exit criteria:** every `examples/*.tn` runs under both backends with
identical output; `tnc build hello.tn` produces a standalone executable.

## M7 — Standard library ⬜ (Phase 7 doc)

- Core: `Option<T>` (نتيجة already exists; `خيار`/`Option` next), iterators,
  sorting, hashing, maps.
- IO: file read/write, stdin, args — designed *trilingually from day one*
  (every name ships in en/ar/fr like `len/طول/longueur`).
- Written in TN619 itself where the language allows — this is the forcing
  function for M4/M5 gaps.

## M8 — Tooling & ecosystem ⬜ (Phase 6 doc)

- **`tnc fmt`** — formatter (canonical layout; understands RTL source).
- **`tnc translate`** — the killer demo: convert a program's *surface*
  between English ⇄ Arabic ⇄ French mechanically (keywords + builtin names
  are tables; identifiers stay as written).
- **LSP server** — diagnostics, hover types, go-to-definition; the playground
  (`tnc serve`) gains live error squiggles.
- **Package manager** skeleton (`tnc add`, lockfile) once modules (M4) exist.

## M9 — Hardening & release ⬜

- Unicode NFC normalization + XID identifiers in the lexer (noted hooks).
- Fuzzing the lexer/parser; differential fuzzing interpreter vs JIT.
- Docs and the tutorial book in all three languages.
- Versioned releases with prebuilt binaries; the playground hosted publicly.

---

## Suggested order

```
M4 generics+traits ──► M4 closures ──► M7 stdlib core
        │                                   │
        └──► M5 references/borrows ◄────────┘
                      │
                      ▼
        M6 backend growth (continuous, feature-by-feature)
                      │
                      ▼
        M8 tooling ──► M9 release
```

Generics first: they unblock both the stdlib and de-special-casing the
compiler. The JIT grows continuously in the background — every new language
feature should land with a "subset or clean error" decision in `jit.rs`.
