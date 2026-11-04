---
Stable-Id: 01M6DCRZ14207ZVE6ZN5YH6J33
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-cli/src/discovery.rs, code:rust/crates/urzua-cli/src/commands/new.rs, test:rust/crates/urzua-cli/tests/new_integration.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/manifest.yaml
---
# 130 — One identity namespace across several directories

## What

A record type may declare more than one directory under one prefix, with numbering (`urzua new`),
resolution and `identity.collision` all spanning every one of those directories as a single
namespace.

## Why

Config load now refuses two types that share a prefix (BUG-173), because a shared prefix guarantees
duplicate ids. But monorepos with per-package decision directories, archived-record folders, and
corpora mid-migration legitimately want one numbering namespace across several directories, and
today a type has exactly one `dir`, so those layouts have no correct configuration.

Related: MILE-98 (declared identity, where the namespace is defined) and BUG-168 (two types sharing
one `dir`, the inverse layout problem).

## Done means

A record type may declare more than one directory under one prefix, and numbering in `urzua new`, identity resolution and `identity.collision` span every one as a single namespace; a fixture with two directories under one prefix gets the next number across both, resolves a reference into either, and reports a collision between them; config load still refuses two types sharing a prefix (`BUG-173`), and the multi-directory declaration is documented in `SPEC-3`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** BUG-173 makes config load refuse a shared prefix, which leaves multi-directory layouts with no correct configuration. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): identity resolution sits under every reference rule, so changing the namespace model after more rules land touches all of them. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. A type declares `extra_dirs` beside `dir`; discovery, `urzua new` (working tree and other refs), `identity.collision`, `filename.identity-pattern`, pointer resolution, the diff gate and `type.dir-matches-nothing` read the directories as one namespace. Exercised in `corpus-i`, not urzua's own config, which has no type spanning directories. **Why:** the work is merged on `main` in the PR that delivers it; checked against this record's What. | **structural** |
