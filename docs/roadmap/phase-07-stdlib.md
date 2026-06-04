# Phase 7 — Standard Library · ⬜ Not started

**Goal:** stdlib modules with security-first defaults.

## ✅ Done
- [x] Two builtins exist in the MVP: `print`/`اطبع`, and the `Result`/`Option`
      types are designed (not yet in stdlib form).

## ⬜ Not done (everything)
- [ ] **core:** `Option`/`خيار`, `Result`/`نتيجة`, primitives, `Iterator`.
- [ ] **collections:** `Vec`, `Map`, `Set`, string utilities.
- [ ] **io:** stdin/stdout/stderr, buffered IO, formatting.
- [ ] **fs:** files, paths, directories.
- [ ] **net:** TCP/UDP, sockets, DNS.
- [ ] **concurrency:** threads, channels (ties to deferred async design).
- [ ] **crypto:** hashing, AEAD, key exchange — constant-time, vetted algorithms.
- [ ] **json / serde:** parsing + (de)serialization.
- [ ] **http / web:** client first, server later.
- [ ] **database:** driver traits + at least one backend.
- [ ] **logging:** structured, leveled.
- [ ] **Security features:** tainted-input tracking in the type system, safe-by-
      default parsers, constant-time primitives.
- [ ] Bilingual API naming strategy (English canonical + Arabic aliases? TBD).

## Forward plan (mapped to Phase 8, Q6)
1. **M12 (M16):** freeze a small `core`, then `collections` + `io` + `fs`.
2. **M13 (M17–18):** `net`, `crypto`, `json`, `logging` — audit crypto early.
Security-oriented features (taint tracking) are designed alongside the type
system, not bolted on later.

**Dependency note:** stdlib needs generics + traits (Phase 2 build, M3) and a
real backend (M5) to be useful — do not start before Q6.
