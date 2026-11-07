---
Stable-Id: 01M6DCRZ14207ZVE6ZN5YH6J33
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig@sha256:a152d1cb2640dc8765b4b7a47685d9ada657d5cde188282a81c615bd630617ab, code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/manifest.yaml, code:rust/crates/urzua-cli/src/commands/new.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_numbers_across_every_directory_of_a_type@sha256:0ab3711ee614da1df16c531e8d2db0f43709377e24a521b5d32fd026ba6d2ec6, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/new-numbers-past-the-highest-id-in-any-directory.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/pointer-resolves-across-the-types-directories.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/one-id-in-two-directories-is-a-collision.yaml, test:rust/crates/urzua-core/src/config.rs#two_types_sharing_a_prefix_are_refused_at_load@sha256:06513e713838feb6655d9493f8e0b0908621ad46f393707de06536a147f03b29, spec:docs/specs/SPEC-3-configuration.md
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

- A record type may declare more than one directory under one prefix. — evidence: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig@sha256:a152d1cb2640dc8765b4b7a47685d9ada657d5cde188282a81c615bd630617ab, code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/manifest.yaml
- Numbering in `urzua new` spans every directory of the type as a single namespace, so a fixture with two directories under one prefix gets the next number across both. — evidence: code:rust/crates/urzua-cli/src/commands/new.rs, test:rust/crates/urzua-cli/tests/new_integration.rs#new_numbers_across_every_directory_of_a_type@sha256:0ab3711ee614da1df16c531e8d2db0f43709377e24a521b5d32fd026ba6d2ec6, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/new-numbers-past-the-highest-id-in-any-directory.yaml
- Identity resolution spans every directory, so a reference into either resolves. — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/pointer-resolves-across-the-types-directories.yaml
- `identity.collision` spans every directory and reports a collision between them. — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/one-id-in-two-directories-is-a-collision.yaml
- Config load still refuses two types sharing a prefix (`BUG-173`). — evidence: test:rust/crates/urzua-core/src/config.rs#two_types_sharing_a_prefix_are_refused_at_load@sha256:06513e713838feb6655d9493f8e0b0908621ad46f393707de06536a147f03b29
- The multi-directory declaration is documented in `SPEC-3`. — evidence: spec:docs/specs/SPEC-3-configuration.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** BUG-173 makes config load refuse a shared prefix, which leaves multi-directory layouts with no correct configuration. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): identity resolution sits under every reference rule, so changing the namespace model after more rules land touches all of them. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. A type declares `extra_dirs` beside `dir`; discovery, `urzua new` (working tree and other refs), `identity.collision`, `filename.identity-pattern`, pointer resolution, the diff gate and `type.dir-matches-nothing` read the directories as one namespace. Exercised in `corpus-i`, not urzua's own config, which has no type spanning directories. **Why:** the work is merged on `main` in the PR that delivers it; checked against this record's What. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
