# TN619 Design — Phase 5: Memory Model

Status: design (no implementation yet). Goal: **safer than C++, easier than Rust,
with zero runtime cost and no garbage collector.**

This document specifies ownership, borrowing, references, mutability, lifetimes
(and their inference — the core differentiator), drop semantics, and thread
safety. Every departure from Rust is justified, and every safety guarantee Rust
provides is preserved.

---

## 0. The core bet (restated)

Rust's difficulty is not accidental: the borrow checker, lifetimes, and
`Send`/`Sync` are the *price* of "memory safety with no GC and no runtime cost."
You cannot delete the hard parts and keep all three guarantees.

TN619's thesis: **keep the guarantees, attack the ergonomics.** Concretely, we
keep Rust's full ownership + borrow model, but:

1. **Infer lifetimes** instead of forcing the programmer to write `'a` (the #1
   cited Rust pain point).
2. **Auto-borrow** aggressively so explicit `&`/`*` nearly disappears at use sites.
3. **Diagnose, don't just reject** — every ownership error explains the concept
   and suggests a fix, bilingually.

We do **not** trade away soundness for any of this.

---

## 1. Ownership

Every value has exactly one **owner** (a binding). When the owner goes out of
scope, the value is destroyed (its `drop` runs). Assigning or passing a
non-`Copy` value **moves** ownership; the source binding becomes invalid.

```tn
fn main() {
    let a = Buffer::new(1024)   // a owns the buffer
    let b = a                    // ownership MOVES to b; a is now invalid
    print(b.len())               // ok
    // print(a.len())            // COMPILE ERROR: use of moved value `a`
}                                // b drops here -> buffer freed exactly once
```

Arabic:
```tn
دالة رئيسي() {
    دع أ = مخزن::جديد(١٠٢٤)
    دع ب = أ                     // الملكية تنتقل إلى ب ؛ أ صار غير صالح
    اطبع(ب.طول())
}
```

**Internal:** move/use-after-move is detected by liveness dataflow over MIR
(Phase 4, Stage 6). No runtime cost — moves are compile-time bookkeeping; the
machine code is a bitwise copy (or nothing, if the optimizer elides it).

**Why this design:** single-ownership + scope-based drop is RAII without a GC.
It gives deterministic destruction (predictable latency — critical for systems
and security code) and makes leaks and double-frees structurally impossible.

**vs Rust:** identical model. The difference is the *error message*: where Rust
says "value used here after move," TN619 adds: "`a` was moved on line 3 because
`Buffer` owns heap memory and can't be silently duplicated. To keep using `a`,
either move it back, borrow it (`&a`), or copy it explicitly (`a.clone()`)."

---

## 2. Copy vs Move types

Small, plain-old-data types (`int`, `bool`, `f64`, `char`, and structs/enums
composed entirely of `Copy` fields) are **`Copy`**: assignment duplicates them
bitwise and the source stays valid. Everything else **moves**.

```tn
let x = 5
let y = x      // int is Copy -> x still usable
print(x)       // ok
```

**`Copy` is inferred, not annotated** (unlike Rust, where you write
`#[derive(Copy)]`). The checker marks a type `Copy` automatically iff all its
fields are `Copy` and it has no custom `drop`. A type with a destructor or heap
ownership is never `Copy`.

**vs Rust:** Rust requires explicit `#[derive(Copy, Clone)]`. We infer it. The
tradeoff: inferring `Copy` means adding a field can silently change a type from
`Copy` to move — a subtle semantic shift. **Mitigation:** for `pub` types, the
checker *warns* when `Copy`-ness flips across an edit (the public contract
changed), and a type may opt out with `!Copy`. Private types infer freely.

---

## 3. Borrowing & references

Instead of giving up ownership, you can **borrow** a value by reference:

- `&T` — a **shared** (read-only) borrow. Many may exist at once.
- `&mut T` — an **exclusive** (read-write) borrow. Exactly one may exist, and no
  shared borrows may coexist with it.

This is the **"shared XOR mutable"** rule — the heart of compile-time data-race
freedom. At any program point, a value has *either* any number of readers *or*
exactly one writer, never both.

```tn
fn longest(a: &str, b: &str) -> &str {   // borrows, doesn't take ownership
    if a.len() > b.len() { a } else { b }
}

fn main() {
    let s1 = "hello"
    let s2 = "hi"
    let r = longest(&s1, &s2)   // both shared-borrowed; s1, s2 still owned by main
    print(r)
}
```

**Auto-borrow (ergonomic departure from Rust):** at method call and argument
sites, TN619 inserts the needed `&`/`&mut` automatically when unambiguous, so
the explicit `&` above is usually optional: `longest(s1, s2)` works, and
`p.sum()` auto-borrows `p` as `&self`. Explicit `&` remains available and is
required where intent is ambiguous. Rust already auto-refs method receivers; we
extend the same inference to function arguments.

**vs Rust:** same borrow rules and guarantees; more inference at use sites.

---

## 4. Mutability

Mutability is layered, and both layers default to immutable:

| What | Immutable (default) | Mutable |
|------|--------------------|---------|
| Binding | `let x` | `var x` |
| Reference | `&T` | `&mut T` |

To mutate through a reference you need **both** a `var`/`&mut` chain: a `&mut T`
borrow of a `var` binding.

```tn
fn push_one(v: &mut Vec<int>) {   // exclusive borrow
    v.push(1)
}

fn main() {
    var nums = Vec::new()   // var -> the binding is mutable
    push_one(&mut nums)     // exclusive borrow handed to push_one
    print(nums.len())       // 1
}
```

**vs Rust:** Rust uses `let mut`; we use `var` (decided in Phase 2). References
are `&`/`&mut` in both. Identical exclusivity semantics.

---

## 5. Lifetimes — and their inference (THE differentiator)

A reference must never outlive the value it points to. Rust enforces this and,
at function boundaries, makes you *name* the relationships with `'a`. TN619
**infers** them.

### 5.1 The model
Lifetimes are **regions**: sets of MIR program points over which a reference must
remain valid. The borrow checker generates constraints (`'r_return ⊆ 'r_input`)
and solves them — exactly Rust's NLL region solver. The novelty is *where* we
require the programmer to participate.

### 5.2 What gets inferred
Consider:
```tn
fn longest(a: &str, b: &str) -> &str { ... }   // returns a or b
```
The checker assigns region variables to each reference, sees the body return
either `a` or `b`, and derives `'ret ⊆ 'a ∧ 'ret ⊆ 'b`. **No annotation needed.**
Rust forces `fn longest<'a>(a: &'a str, b: &'a str) -> &'a str`.

Target: **a typical TN619 program has zero visible lifetime annotations.**

### 5.3 The hard case (why this is a *bet*, not a freebie)
Some signatures are genuinely ambiguous — the body could relate the output to
different inputs, and the choice is part of the *API contract*, not an
implementation detail. Worse, inferring signature lifetimes from the body means
**a body change can silently alter the public API's lifetime contract**, breaking
downstream callers without any signature edit. That is unacceptable for a stable
ecosystem.

### 5.4 The boundary decision (private-infer / public-explicit)
- **Private/local functions:** infer lifetimes freely from the body. The body
  *is* the contract; there are no external callers to surprise.
- **`pub` functions:** infer when there is exactly one input reference (the
  output can only borrow from it — unambiguous and stable). When there are
  multiple input references *and* the output borrows, **require explicit
  lifetimes** (`&'a`/`&'حياة`). The annotation is then a deliberate, visible part
  of the public contract — which is exactly when you *want* it visible.

This yields "zero annotations in the overwhelming majority of code" while keeping
public APIs honest and stable. It is the single most important rule in this
document.

```tn
// pub, single input ref -> inferred, no annotation:
pub fn first_word(s: &str) -> &str { ... }

// pub, multiple input refs + borrowed output -> explicit required:
pub fn longest<'a>(a: &'a str, b: &'a str) -> &'a str { ... }
```

### 5.5 Alternatives considered
1. **Infer everything, even public APIs** — maximum ergonomics, but API drift
   (5.3) makes it unsound as a stability contract. Rejected.
2. **Require explicit lifetimes for all reference-returning signatures (Rust)** —
   stable and simple to implement, but keeps Rust's #1 pain. Rejected as the
   default (it *is* our fallback for the ambiguous public case).
3. **Private-infer / public-explicit-when-ambiguous (chosen)** — best balance:
   sound, stable, and annotation-free in ~80–95% of real code.

---

## 6. Drop / RAII

When an owner goes out of scope, its `drop` runs (custom `drop`/`أسقط` method if
defined, then fields recursively). Deterministic, ordered (reverse declaration),
zero-cost (no GC, no finalizer thread). Moved-out values are not dropped at the
source (the move transferred the obligation).

The compiler tracks drop obligations per path in MIR and inserts drop calls only
where a value is actually live and owned — "drop elaboration," same as Rust.

---

## 7. Thread safety

Data-race freedom extends to threads via two auto-derived marker traits:
- **`Shareable`** (Rust's `Sync`): a `&T` may be sent across threads.
- **`Sendable`** (Rust's `Send`): a `T` may be moved to another thread.

Both are **inferred** structurally (a type is `Sendable` iff all fields are).
Types with thread-unsafe interior mutability are not `Shareable`, so the
"shared XOR mutable" rule plus these markers make data races a **compile error**,
not a runtime crash. Names are friendlier than `Send`/`Sync`; semantics identical.

Note: this interacts with the (deferred) concurrency/async model. We commit only
to the marker-trait foundation here.

---

## 8. Explicit list of simplifications vs Rust

| Rust friction | TN619 |
|---------------|-------|
| Write `'a` lifetimes | Inferred (explicit only for ambiguous `pub` fns) |
| `#[derive(Copy, Clone)]` | `Copy` inferred; `clone()` still explicit |
| Explicit `&`/`*` at many sites | Auto-borrow / auto-deref where unambiguous |
| `iter()`/`iter_mut()`/`into_iter()` | Borrow-by-default `for x in xs` (Phase 2) |
| `Send`/`Sync` jargon | `Sendable`/`Shareable`, inferred |
| Terse borrow-check errors | Tiered, bilingual, fix-suggesting diagnostics |

**What we deliberately keep (non-negotiable for soundness):** single ownership,
move semantics, shared-XOR-mutable, no GC, deterministic drop, the region-based
borrow checker. We simplify the *surface*, never the *guarantees*.

---

## 9. Implementation plan (sound-first)

1. **Ownership + move checking** over MIR (liveness). Ship this first; it catches
   use-after-move with no lifetime reasoning.
2. **Borrow checking** (shared-XOR-mutable) via dataflow. Initially require more
   explicit `&`/`&mut`; add auto-borrow after.
3. **Region inference** for *local* functions only. Validate on a large test
   suite of known-good/known-bad programs.
4. **Extend inference to `pub` functions** with the single-input rule; require
   explicit lifetimes for the ambiguous multi-input case.
5. **Drop elaboration**, then **`Copy`/`Sendable`/`Shareable`** inference.

**Golden rule:** a conservative checker that *rejects some valid programs* is
acceptable and shippable; an *unsound* checker that accepts an invalid one is
never acceptable. We tighten ergonomics over time from a sound base.

---

## 10. Relationship to the current MVP

The tree-walking interpreter (M0–M0.3) uses value semantics with cloning, which
is why `&self` is sound there but `&mut self` cannot persist mutations. This
memory model is what a real backend + borrow checker will implement; `&mut self`
becomes correct once references are real (step 2 above). Until then the
interpreter remains a semantics oracle for everything *except* mutation-through-
reference.
```
