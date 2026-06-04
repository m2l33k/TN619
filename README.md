# TN619

A modern systems programming language with **bilingual (English + Arabic) source syntax**,
Rust-class safety and performance goals, and a beginner-friendly design.

English and Arabic keywords compile to the **same AST** — the language surface is
erased at the lexer boundary, so the type system, semantics, and tooling are monolingual.

```
fn main() {                 دالة رئيسي() {
    let age = 20                دع العمر = ٢٠
    if age > 18 {              اذا العمر > ١٨ {
        print("Adult")            اطبع("بالغ")
    }                          }
}                           }
```
Both of the above produce identical tokens, AST, and output.

## Documentation

- [docs/roadmap/](docs/roadmap/) — **per-phase roadmap** (one file per phase 1–10:
  what's done, what's not, and the forward plan). Start at
  [docs/roadmap/README.md](docs/roadmap/README.md).
- [docs/ROADMAP.md](docs/ROADMAP.md) — consolidated 24-month build plan.
- [docs/design/05-memory-model.md](docs/design/05-memory-model.md) — ownership, borrowing, and lifetime inference.
- [docs/design/02-language-features.md](docs/design/02-language-features.md) — numeric tower, string interpolation, closures, async direction.
- [docs/design/positioning.md](docs/design/positioning.md) — brand, competitive positioning, target segments.

## Status: MVP (M0)

The bootstrap compiler `tnc` is a **tree-walking interpreter** (LLVM/Cranelift codegen
comes later). It currently supports: variables (`let`/`var`), arithmetic, functions,
`print`, `if`/`else` (as an expression), `while`, `for ... in a..b`, recursion,
**structs**, **enums** (unit + tuple variants), **pattern matching** (`match` with
literal/binding/wildcard/variant patterns), field access, **methods & associated
functions** (`impl` blocks with `self`/`&self`), and
**bilingual keywords + Arabic-Indic / Persian digits** — in English, Arabic, or
mixed source.

A **static type checker** runs before execution: it infers `let` types, checks function
signatures / struct fields / enum payloads against explicit annotations, and performs
**compile-time `match` exhaustiveness checking** (a missing enum variant is a compile
error, with the missing variants named).

Secure-by-default touches already present: immutable bindings by default, no truthiness
(conditions must be `bool`), **checked integer arithmetic** (overflow is an error), and
**Trojan-Source defense** (bidirectional control characters are rejected in source).

Primitive type names: `int`/`عدد`, `bool`/`منطقي`, `str`/`نص`. Function parameters and
return types are explicit (`fn f(n: int) -> int`); local `let` types are inferred.

## Build & run

Requires Rust (stable). No other dependencies.

```sh
cargo build
cargo run -- run examples/adult_en.tn      # English
cargo run -- run examples/adult_ar.tn      # Arabic
cargo run -- run examples/mixed.tn         # mixed
cargo run -- run examples/polyglot.tn      # English + Arabic combined in ONE program
cargo run -- run examples/interp_en.tn     # string interpolation (English)
cargo run -- run examples/interp_ar.tn     # string interpolation (Arabic)
cargo run -- run examples/shapes_en.tn     # structs + enums + match (English)
cargo run -- run examples/shapes_ar.tn     # structs + enums + match (Arabic)
cargo run -- run examples/points_en.tn     # methods + associated functions (English)
cargo run -- run examples/points_ar.tn     # methods + associated functions (Arabic)
cargo run -- check examples/adult_ar.tn    # type-check only, no execution
cargo run -- run examples/bad_type.tn      # REJECTED: bool condition required
cargo run -- run examples/bad_exhaustive.tn# REJECTED: non-exhaustive match
cargo run -- tokens examples/mixed.tn      # dump the neutral token stream
```

## Layout

```
TN619/
├── Cargo.toml              # workspace root
├── compiler/tnc/           # bootstrap compiler (MVP)
│   └── src/
│       ├── token.rs        # tokens + the bilingual keyword map (the core mechanism)
│       ├── lexer.rs        # bilingual lexer (Arabic digit folding, bilingual comma)
│       ├── ast.rs          # language-neutral AST
│       ├── parser.rs       # recursive descent + Pratt
│       ├── typeck.rs       # static type checker + match exhaustiveness
│       ├── interp.rs       # tree-walking interpreter (temporary backend)
│       └── main.rs         # `tnc` CLI
└── examples/               # bilingual sample programs
```

## Deferred (documented in source / roadmap)

- Unicode NFC normalization + XID-based identifiers (need external crates; the
  hooks are noted in `lexer.rs`). _(Trojan-Source/bidi rejection is implemented.)_
- Ownership / borrow checker + lifetime inference (designed in
  `docs/design/05-memory-model.md`).
- References + `&mut self`, arrays/`Vec`, `f64`, closures, generics, traits,
  modules, error handling, macros (designed; not yet built).
- LLVM (release) + Cranelift (debug) backends behind a backend boundary.
- The full per-stage crate split (Phase 9 monorepo).

## License

Dual-licensed under either of **MIT** ([LICENSE-MIT](LICENSE-MIT)) **or**
**Apache-2.0** ([LICENSE-APACHE](LICENSE-APACHE)) at your option — the
Rust-ecosystem convention. Unless you state otherwise, any contribution you
submit shall be dual-licensed as above, without additional terms.
