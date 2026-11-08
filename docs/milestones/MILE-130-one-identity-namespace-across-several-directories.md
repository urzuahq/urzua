---
Stable-Id: 01M6DCRZ14207ZVE6ZN5YH6J33
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig@sha256:a152d1cb2640dc8765b4b7a47685d9ada657d5cde188282a81c615bd630617ab, code:rust/crates/urzua-cli/src/discovery.rs@sha256:5f489efc9a95c213874196d2bb8fc5592fbecd07e4787cbe1a26f1ebbc6c7ab2, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/manifest.yaml@sha256:84ceadbe8c758c226373e8dcca718b8bc3d65e2e2fd7cd353cf11c37fef4135c, code:rust/crates/urzua-cli/src/commands/new.rs@sha256:5b61c497ff7e8ea4e30f9671a53477e21427637853ef4871324122e767e70b46, test:rust/crates/urzua-cli/tests/new_integration.rs#new_numbers_across_every_directory_of_a_type@sha256:0ab3711ee614da1df16c531e8d2db0f43709377e24a521b5d32fd026ba6d2ec6, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/new-numbers-past-the-highest-id-in-any-directory.yaml@sha256:168fde4d1bf1c0845e4fc7098c2d4a68f945a0fd9da878cf4d9e4daeb5a658c8, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/pointer-resolves-across-the-types-directories.yaml@sha256:ea223cbab6cbc7c04e8cfffc56c401614e65999b8a45e27092cf053132dd3867, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/one-id-in-two-directories-is-a-collision.yaml@sha256:2d37df7b16efe12cd300592ebdc09e6c780b2f5424ba413cfbb6ea2fba95cc05, test:rust/crates/urzua-core/src/config.rs#two_types_sharing_a_prefix_are_refused_at_load@sha256:06513e713838feb6655d9493f8e0b0908621ad46f393707de06536a147f03b29, spec:docs/specs/SPEC-3-configuration.md
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

- A record type may declare more than one directory under one prefix. — evidence: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig@sha256:a152d1cb2640dc8765b4b7a47685d9ada657d5cde188282a81c615bd630617ab, code:rust/crates/urzua-cli/src/discovery.rs@sha256:5f489efc9a95c213874196d2bb8fc5592fbecd07e4787cbe1a26f1ebbc6c7ab2, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/manifest.yaml@sha256:84ceadbe8c758c226373e8dcca718b8bc3d65e2e2fd7cd353cf11c37fef4135c
- Numbering in `urzua new` spans every directory of the type as a single namespace, so a fixture with two directories under one prefix gets the next number across both. — evidence: code:rust/crates/urzua-cli/src/commands/new.rs@sha256:5b61c497ff7e8ea4e30f9671a53477e21427637853ef4871324122e767e70b46, test:rust/crates/urzua-cli/tests/new_integration.rs#new_numbers_across_every_directory_of_a_type@sha256:0ab3711ee614da1df16c531e8d2db0f43709377e24a521b5d32fd026ba6d2ec6, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/new-numbers-past-the-highest-id-in-any-directory.yaml@sha256:168fde4d1bf1c0845e4fc7098c2d4a68f945a0fd9da878cf4d9e4daeb5a658c8
- Identity resolution spans every directory, so a reference into either resolves. — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/pointer-resolves-across-the-types-directories.yaml@sha256:ea223cbab6cbc7c04e8cfffc56c401614e65999b8a45e27092cf053132dd3867
- `identity.collision` spans every directory and reports a collision between them. — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/one-id-in-two-directories-is-a-collision.yaml@sha256:2d37df7b16efe12cd300592ebdc09e6c780b2f5424ba413cfbb6ea2fba95cc05
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
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 6 locators pinned, 6 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/discovery.rs@sha256:eeef78dc2bb8, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/manifest.yaml@sha256:84ceadbe8c75, rust/crates/urzua-cli/src/commands/new.rs@sha256:5b61c497ff7e, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/new-numbers-past-the-highest-id-in-any-directory.yaml@sha256:168fde4d1bf1, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/pointer-resolves-across-the-types-directories.yaml@sha256:ea223cbab6cb, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-i/expectations/one-id-in-two-directories-is-a-collision.yaml@sha256:2d37df7b16ef Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs still discloses every non-absent exclusion in a rule's population (MILE-111), catalog.rs still registers header.case-variant-key (MILE-113), export.rs still emits each record as plain JSON and generate-dashboard.py still reads it (MILE-115), the Makefile and checks.yml still run the zero-warning gate (MILE-116), AGENTS.md and config.rs still state and read the declared-adopter premise (MILE-118), diff_scope.rs still selects the records a --base run examines (MILE-123), boundary.rs, back_pointer.rs and the corpus-h expectation still implement and prove the declared published-path boundary (MILE-125), the claim_expiry.rs, reverify.rs and commands/reverify.rs files and the changeset still implement ADR-97 beside embodiment.rs, discovery.rs and fix.rs (MILE-129), discovery.rs, new.rs and the corpus-i files still give one identity namespace across directories (MILE-130), and the rules.rs module index and the per-family rule modules still exist as the split MILE-131 describes. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` eeef78dc2bb8 -> 5f489efc9a95. Reason: rust/crates/urzua-cli/src/discovery.rs changed only in compute_claim_measures, which now blames the first evidence line through the shared evidence_anchor_line instead of the Realized-by field line; the code this record cites is not touched by that change, so the claim stands (diffed, not re-read in full). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
