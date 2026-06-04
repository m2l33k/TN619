# TN619 Design — Phase 2 completion: remaining language features

Specs for the four Phase-2 items that were designed informally or deferred:
the **numeric type tower**, **string interpolation**, **closures**, and
**async / concurrency**. These are design specs; only a subset is implemented in
the MVP (noted per section).

---

## 1. Numeric type tower

The MVP has a single `int`/`عدد` (an `i64`). The full tower:

| Category | Types | Arabic | Default |
|----------|-------|--------|---------|
| Signed integer | `i8 i16 i32 i64 isize` | `ص٨ ص١٦ ص٣٢ ص٦٤ صحجم` | — |
| Unsigned integer | `u8 u16 u32 u64 usize` | `ط٨ ط١٦ ط٣٢ ط٦٤ طحجم` | — |
| Float | `f32 f64` | `ع٣٢ ع٦٤` | — |
| Convenience aliases | `int` = `i64`, `uint` = `u64`, `float` = `f64` | `عدد طبيعي عائم` | yes |

Naming mnemonic: **ص** = صحيح (signed integer), **ط** = طبيعي (natural ⇒ unsigned),
**ع** = عشري (decimal ⇒ float), `حجم` = size (pointer-sized).

**Inference & defaulting** (matches Rust): an unsuffixed integer literal gets a
`{integer}` type variable; if unconstrained at the end of inference it defaults to
`i64`. Float literals default to `f64`. Suffix literals are allowed: `10u8`,
`3.0f32` (`١٠ط٨`).

**Overflow:** checked by default (overflow is an error, as in the MVP). Release
builds may opt into wrapping per-operation via explicit methods
(`a.wrapping_add(b)`), never silent UB.

**Casts:** explicit only, via `as`/`كـ`: `x as i64`, `n كـ ع٦٤`. No implicit
numeric coercion (prevents a class of bugs and keeps inference predictable).

**MVP status:** only `int`/`عدد` (i64) implemented. The rest is forward design.

---

## 2. String interpolation

```tn
let name = "Sara"
let age = 20
print("Hello, {name} — you are {age}")     // Hello, Sara — you are 20
print("next year: {age + 1}")               // arbitrary expression
print("a literal brace: {{")                 // escaped -> a literal brace: {
```
Arabic:
```tn
اطبع("مرحبا، {name} عمرك {age}")
```

**Lexing:** an interpolated string is scanned into a sequence of parts —
`Lit(text)` and `Expr(token-range)` — splitting on unescaped `{ … }`. `{{` and
`}}` are literal braces. Each `Expr` part is re-lexed/parsed as a normal
expression.

**Desugaring (internal):** `"a {x} b"` lowers to a call to a `format`/`Display`
builder, conceptually `concat(["a ", to_str(x), " b"])`. Every interpolated value
must implement `Display`/`عرض`. Zero hidden allocation beyond building the result
string.

**Internal representation:**
```
Expr::StrInterp { parts: Vec<StrPart> }
enum StrPart { Lit(String), Expr(Box<Expr>) }
```

**MVP status:** not implemented (strings are plain literals). This is the spec.

---

## 3. Closures (anonymous functions)

```tn
let add  = |a, b| a + b                      // inferred from use
let inc   = |x: int| -> int { x + 1 }         // annotated
let nums = [1, 2, 3]
let doubled = nums.map(|n| n * 2)             // passed to higher-order fn
```
Arabic:
```tn
دع زد = |س| س + ١
دع المضاعف = الأرقام.تحويل(|ن| ن * ٢)
```

**Capture & ownership** (ties into Phase 5):
- By default a closure **borrows** the variables it uses (`&`), for the closure's
  lifetime — the least-surprising default.
- Prefix `move`/`نقل` to **capture by value** (move/copy): `move |x| x + base`.
- Capture mode per variable is inferred from how the body uses it (read → `&`,
  mutate → `&mut`, consumed → move), with `move` forcing by-value. Same model as
  Rust, but the annotations are inferred so beginners rarely write them.

**Types.** A closure's type is anonymous; it is accepted wherever a function type
is expected. Function/closure type syntax:
```tn
fn apply(f: fn(int) -> int, x: int) -> int { f(x) }
```
For closures that capture, TN619 exposes **one** callable trait `Callable`/`قابل_للنداء`
(rather than Rust's three-way `Fn`/`FnMut`/`FnOnce`) by default; the distinction is
inferred and only surfaced in advanced/`pub` API positions when it matters.
*Rationale:* removes a notorious Rust beginner wall while keeping the underlying
guarantees.

**Internal representation:**
```
Expr::Closure { params: Vec<Param>, ret: Option<Type>, body: Block, captures: Inferred }
```

**MVP status:** not implemented. Forward design; depends on the type checker
supporting function types + the borrow checker for capture analysis.

---

## 4. Async / Concurrency (the deferred deep-dive)

Concurrency is where ambitious languages stumble. Three honest options:

| Option | Model | Pros | Cons |
|--------|-------|------|------|
| **A. async/await (Rust-style)** | `async fn` returns a future; `.await`; runtime schedules | zero-cost, no GC, maximal control | **function coloring** (async infects call chains), `Pin`/lifetime complexity — the opposite of beginner-friendly |
| **B. Lightweight tasks + channels (Go-style)** | `spawn`/`اطلق` a task; communicate over channels | very easy, no coloring, great mental model | needs a runtime/scheduler; classic green-threads have a runtime cost & FFI friction |
| **C. Structured concurrency (nurseries)** | tasks must be spawned within a scope that awaits all children; channels for comms | easy *and* safe (no leaked/orphan tasks), composable cancellation | newer paradigm, fewer references to copy from |

**Recommendation: C — structured concurrency, with B's ergonomics.**
```tn
// sketch (forward design, not final syntax)
async رئيسي() {
    nursery |n| {                 // a scope that owns its child tasks
        n.spawn(|| fetch("a"))    // child tasks
        n.spawn(|| fetch("b"))
    }                             // scope can't exit until both finish/cancel
}
```
Why C:
- **No orphaned tasks / no leaks** — a task cannot outlive its scope, which makes
  cancellation and error propagation tractable (the hardest part of async).
- **Beginner-friendly** — the structure mirrors ordinary block scoping.
- It can be *implemented on top of* async/await internally, so we keep zero-cost
  potential while hiding the coloring/`Pin` complexity behind the nursery API.

### Finalized surface syntax (bilingual)
The blessed surface for v1 is the structured API. Keywords:

| English | Arabic | Meaning |
|---------|--------|---------|
| `async` | `متزامن` | marks a function that may suspend |
| `await` | `انتظر` | await a suspendable result |
| `nursery` | `حضانة` | a scope owning child tasks |
| `spawn` | `أطلق` | start a child task within a nursery |
| `channel` | `قناة` | typed message channel |

```tn
async fn fetch_all() -> int {            // متزامن دالة …
    nursery |n| {                        // حضانة
        n.spawn(|| fetch("a"))           // أطلق
        n.spawn(|| fetch("b"))
    }                                    // scope joins/cancels all children here
    42
}
```
Raw `async`/`await` exist but the **nursery** is the recommended surface; the
nursery guarantees no task outlives its scope (no leaks; structured cancellation).

**Decision: async is its own dedicated future phase, NOT part of Phase 2's scope.**
Per the Phase 1 non-goals it is *intentionally not a launch feature*. The surface
syntax above is finalized; the semantics (runtime, scheduler, exact cancellation
and coloring behavior) are specified in that future phase. This is a deliberate
scoping decision, not an omission.

---

## Summary of MVP vs designed

| Feature | Designed | In MVP |
|---------|----------|--------|
| Numeric tower | ✅ | only `int`/`عدد` (f64/sized ints pending) |
| String interpolation | ✅ | ✅ **implemented** (`examples/interp_*.tn`) |
| Closures | ✅ | ❌ (needs fn types + capture analysis) |
| Async/concurrency | ✅ surface finalized; semantics = dedicated future phase | ❌ (not a launch feature) |
