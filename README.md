# TN619

A modern systems programming language with **trilingual (English + Arabic + French)
source syntax**, Rust-class safety and performance goals, and a beginner-friendly design.

English, Arabic, and French keywords compile to the **same AST** — the language surface
is erased at the lexer boundary, so the type system, semantics, and tooling are monolingual.

```
fn main() {                 دالة رئيسي() {              fonction principal() {
    let age = 20                دع العمر = ٢٠               soit âge = 20
    if age > 18 {              اذا العمر > ١٨ {             si âge > 18 {
        print("Adult")            اطبع("بالغ")                 affiche("Adulte")
    }                          }                           }
}                           }                           }
```
All three of the above produce identical tokens, AST, and output — and the three
surfaces can be **mixed freely in one program** (see `examples/trilingue.tn`).

## Documentation

- [docs/roadmap/](docs/roadmap/) — **per-phase roadmap** (one file per phase 1–10:
  what's done, what's not, and the forward plan). Start at
  [docs/roadmap/README.md](docs/roadmap/README.md).
- [docs/roadmap/NEXT-PHASES.md](docs/roadmap/NEXT-PHASES.md) — **what's next
  (M4+)**: generics/traits, closures, modules, references + borrow checker,
  backend growth, stdlib, tooling, release.
- [docs/ROADMAP.md](docs/ROADMAP.md) — consolidated 24-month build plan.
- [docs/design/05-memory-model.md](docs/design/05-memory-model.md) — ownership, borrowing, and lifetime inference.
- [docs/design/02-language-features.md](docs/design/02-language-features.md) — numeric tower, string interpolation, closures, async direction.
- [docs/design/positioning.md](docs/design/positioning.md) — brand, competitive positioning, target segments.

## Status: M3 (arrays, Result/`?`, `&mut self`, ownership, native JIT)

The bootstrap compiler `tnc` runs programs through a **tree-walking interpreter**
(the reference backend) and can compile the int/bool subset to **native machine
code** via a Cranelift JIT (`tnc jit`). The language currently supports: variables
(`let`/`var`), arithmetic, functions, `print`, `if`/`else` (as an expression),
`while`, `for i in a..b`, `for x in arr`, recursion, **arrays** (`[T]` with
literals, indexing, `len`/`push`/`pop`), **structs**, **enums** (unit + tuple
variants), **`Result<T, E>` + the `?` operator** (with `؟` as its Arabic
spelling), **pattern matching** (`match` with literal/binding/wildcard/variant
patterns, incl. `Ok`/`Err`), field access, **methods & associated functions**
(`impl` blocks with `self`/`&self`/**`&mut self`**, with in-place mutation and
write-back), and **trilingual keywords + Arabic-Indic / Persian digits** — in
English, Arabic, French, or mixed source.

A **static type checker** runs before execution: it infers `let` types, checks function
signatures / struct fields / enum payloads against explicit annotations, performs
**compile-time `match` exhaustiveness checking** (a missing enum variant is a compile
error, with the missing variants named), enforces **compile-time mutability** (`var`
vs `let`, `&mut self` vs `&self`), and runs an **ownership move checker**: non-Copy
values (str, structs, enums, arrays, Result) move on `let`-init, argument passing,
constructor payloads, `match`, and by-value method calls; use-after-move, moves
inside loops, and moving `self` out of a borrowed method are compile errors.
`.clone()` / `انسخ()` / `.cloner()` is the explicit copy.

Secure-by-default touches already present: immutable bindings by default, no truthiness
(conditions must be `bool`), **checked integer arithmetic** (overflow is an error in
the interpreter), **bounds-checked array indexing**, and **Trojan-Source defense**
(bidirectional control characters are rejected in source; UTF-8 BOM is tolerated).

Primitive type names: `int`/`عدد`/`entier`, `bool`/`منطقي`/`booléen`,
`str`/`نص`/`chaîne`, `float`/`عائم`/`flottant`. Function parameters and return
types are explicit (`fn f(n: int) -> int`); local `let` types are inferred.
French keywords with accents also accept accent-stripped spellings
(`énum`/`enum`, `chaîne`/`chaine`, `booléen`/`booleen`).

| en | ar | fr | | en | ar | fr |
|---|---|---|---|---|---|---|
| `let` | `دع` | `soit` | | `true` | `صحيح` | `vrai` |
| `var` | `متغير` | `variable` | | `false` | `خطأ` | `faux` |
| `fn` | `دالة` | `fonction` | | `struct` | `هيكل` | `structure` |
| `if` | `اذا` | `si` | | `enum` | `تعداد` | `énum` |
| `else` | `وإلا` | `sinon` | | `match` | `طابق` | `selon` |
| `while` | `طالما` | `tantque` | | `impl` | `تطبيق` | `implémente` |
| `for` | `لكل` | `pour` | | `as` | `كـ` | `comme` |
| `in` | `في` | `dans` | | `self` | `الذات` | `soi` |
| `return` | `أرجع` | `retourne` | | `print` | `اطبع` | `affiche` |

Entry point: `fn main()` / `دالة رئيسي()` / `fonction principal()`.

Builtin types & methods: arrays `[T]` with `len`/`طول`/`longueur`,
`push`/`أضف`/`ajoute`, `pop`/`اسحب`/`retire`; `Result<T, E>` /
`نتيجة<ق، خ>` / `Résultat<T, E>` with constructors `Ok`/`نجاح` and
`Err`/`فشل`/`Erreur`, propagated by `?`/`؟`; `clone`/`انسخ`/`cloner` on any
value.

## Build & run

Requires Rust (stable). The interpreter and playground are dependency-free;
`tnc jit` (native codegen) uses Cranelift.

```sh
cargo build
cargo run -- run examples/adult_en.tn      # English
cargo run -- run examples/adult_ar.tn      # Arabic
cargo run -- run examples/adult_fr.tn      # French
cargo run -- run examples/mixed.tn         # mixed
cargo run -- run examples/polyglot.tn      # English + Arabic combined in ONE program
cargo run -- run examples/trilingue.tn     # English + Arabic + French in ONE program
cargo run -- run examples/shapes_fr.tn     # structs + énums + selon (French)
cargo run -- run examples/points_fr.tn     # méthodes + fonctions associées (French)
cargo run -- run examples/arrays_en.tn     # arrays: indexing, push/pop, iteration
cargo run -- run examples/result_en.tn     # Result<T, E> + the ? operator
cargo run -- run examples/counter_mut.tn   # &mut self: in-place mutation (3 surfaces)
cargo run -- run examples/ownership.tn     # moves + .clone()
cargo run -- jit examples/jit_fib.tn       # NATIVE machine code via Cranelift
cargo run -- run examples/interp_en.tn     # string interpolation (English)
cargo run -- run examples/interp_ar.tn     # string interpolation (Arabic)
cargo run -- run examples/floats_en.tn     # f64 + numeric casts (English)
cargo run -- run examples/floats_ar.tn     # f64 + numeric casts (Arabic)
cargo run -- serve                          # web playground at http://127.0.0.1:8080
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
│       ├── token.rs        # tokens + the trilingual keyword map (the core mechanism)
│       ├── lexer.rs        # trilingual lexer (Arabic digit folding, bilingual comma)
│       ├── ast.rs          # language-neutral AST
│       ├── parser.rs       # recursive descent + Pratt
│       ├── typeck.rs       # type checker + exhaustiveness + mutability + move checker
│       ├── interp.rs       # tree-walking interpreter (reference backend)
│       ├── jit.rs          # Cranelift JIT (native codegen, int/bool subset)
│       └── main.rs         # `tnc` CLI
└── examples/               # trilingual sample programs
```

## Deferred (documented in source / roadmap)

- Unicode NFC normalization + XID-based identifiers (need external crates; the
  hooks are noted in `lexer.rs`). _(Trojan-Source/bidi rejection is implemented.)_
- General references (`&T` beyond `self`) + lifetime inference (designed in
  `docs/design/05-memory-model.md`). The move checker tracks whole variables;
  field projections are implicit clones for now.
- Closures, generics, traits, modules, macros (designed; not yet built).
- Growing the native (Cranelift) backend beyond the int/bool subset:
  strings, structs, arrays, Result; overflow traps to match the interpreter.
- The full per-stage crate split (Phase 9 monorepo).

## License

Dual-licensed under either of **MIT** ([LICENSE-MIT](LICENSE-MIT)) **or**
**Apache-2.0** ([LICENSE-APACHE](LICENSE-APACHE)) at your option — the
Rust-ecosystem convention. Unless you state otherwise, any contribution you
submit shall be dual-licensed as above, without additional terms.
