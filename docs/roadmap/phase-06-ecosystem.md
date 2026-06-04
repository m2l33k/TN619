# Phase 6 — Ecosystem & Tooling · ⬜ Not started

**Goal:** the developer-experience layer.

## ✅ Done
- [x] Tool names reserved: `tnpkg`, `tn build`, `tnfmt`, `tnlint`, `tndoc`, `tnls`.
- [x] CLI: `tnc run|check|tokens|serve`.
- [x] **Web playground** (`tnc serve`) — dependency-free HTTP server (`server.rs`,
      std only) serving an in-browser editor that runs TN619 (EN/AR/mixed) via a
      `/run` endpoint. The shareable demo. _(Hosted/WASM version is a follow-up.)_

## ⬜ Not done (everything)
- [ ] **`tnpkg`** — package manager: manifest format, registry, dependency
      resolution, lockfile, semantic versioning.
- [ ] **`tn build`** — build system mapping the filesystem to the module graph;
      debug (Cranelift) vs release (LLVM) modes.
- [ ] **`tnfmt`** — formatter, including bilingual canonical formatting rules.
- [ ] **`tnlint`** — linter (naming conventions, `panic`-in-library, glob imports,
      confusable identifiers).
- [ ] **`tndoc`** — documentation generator (bilingual output).
- [ ] **`tnls`** — Language Server (LSP) built on the salsa query DB.
- [ ] Debugger integration (DWARF emission + LLDB/GDB).
- [ ] Editor extensions (VS Code first).

## Forward plan (mapped to Phase 8, Q5)
1. **M9 (M13):** `tnpkg` + `tn build` (depends on the module system being real).
2. **M10 (M14):** `tnfmt` + `tnlint`.
3. **M11 (M15):** `tnls` — cheap IF the salsa refactor (M8) is done, since the LSP
   reuses the query engine.
Debugger + editor extensions follow after `tnls`.

**Dependency note:** most of Phase 6 depends on Phase 4's salsa architecture (M8)
and a real build/module system — sequence accordingly.
