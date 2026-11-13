---
Stable-Id: 01M6DEN9P605CKC9GV4VVTE0PV
Status: Done
Phase: '1'
Track: write-path
---
# 134 — Rename a corpus's record files to the type's declared filename convention

## What

`urzua migrate filenames`: rename a corpus's record files to the filename convention its type's `identity.pattern` declares (`TYPE-NNNN-slug.md` for this repository's `ADR-36` corpus), and rewrite relative links and path mentions to the renamed files, reporting every rewrite. Dry run by default.

## Why

`RFC-46` was rejected in favour of one convention: a corpus in another shape is renamed rather than tolerated. The first such adoption is renamed by a one-off script inside its own switchover change, so the tool is later work; it becomes worth building once adopting a second corpus repeats the same rename.

## Done means

- `urzua migrate filenames` renames a corpus's record files to the convention the type's `identity.pattern` declares, never a hardcoded shape. — evidence: code:rust/crates/urzua-core/src/migrate_filenames.rs#target_name@sha256:1bc72f46cccf67ac10c96eb3c104e82e1a858ff48a1143b5d25c3b49693dd01b, test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#the_target_follows_the_declared_pattern_never_a_hardcoded_shape@sha256:573ae2b9ae04383272c5468104367eefe3639932ef7cf91f6e49356757aeaf68
- It rewrites relative links and path mentions to the renamed files and reports every rewrite. — evidence: code:rust/crates/urzua-core/src/migrate_filenames.rs#rewrite_citations@sha256:fd4390736e8bb6e63ad0617bed88cc0c7b2fec3ae36ceb6bac77c644e3cd94d2, test:rust/crates/urzua-core/src/migrate_filenames.rs#citations_resolve_against_the_file_or_the_root_and_nothing_else@sha256:7c47b3b9fe9cb0a430f0dc74544015a6b30712bf292668e6473e277c37bc493b, test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#apply_renames_keeps_identity_rewrites_citations_and_check_passes@sha256:5853849357d777527ff1371181e0ec0109b682965b66ec389235360dcf06c739
- It is a dry run by default. — evidence: test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#a_dry_run_lists_every_rename_and_citation_and_writes_nothing@sha256:6a1f0b05160b59a34847af5020ecab6ca5a7f06053acc3538e0243172fe71dcc
- A second applied run changes nothing. — evidence: test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#a_second_applied_run_changes_nothing@sha256:4629c9665c064a6bce64f7c766c556c0a0b8383884b31b4f720aacb13c368094
- A fixture with bare zero-padded filenames renames to `TYPE-NNNN-slug.md` and then passes `check`. — evidence: test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#apply_renames_keeps_identity_rewrites_citations_and_check_passes@sha256:5853849357d777527ff1371181e0ec0109b682965b66ec389235360dcf06c739
- It is documented in `SPEC-14`. — evidence: spec:docs/specs/SPEC-14-urzua-migrate.md

## References

- `RFC-46` -- the rejected alternative.
- `ADR-36` -- the filename convention.
- `MILE-120` -- the rule that reports off-convention files.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
> | 2026-11-02 | Target follows the declared pattern. **Why:** The tool must not hardcode one filename shape. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 2 to Phase 1; `Status` unchanged. The switchover (`MILE-102`) renames the adopter's files with a one-off script instead, so the reusable tool is no longer on the Phase 0 path. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-13 | Built: `urzua migrate filenames` (`SPEC-14`), dry run unless `--apply`; every Done means item is evidenced and `Status` follows from them. The command name and `--apply` flag follow `migrate ids` and `fix`; the record named the command, not its flags. | **substantive** |
> | 2026-11-13 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (6 of 6 items resolved) computes to Specified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/migrate_filenames.rs#target_name` (lines 101-159) 4b2083eff42c -> 1bc72f46cccf. Reason: Read the diff of migrate_filenames.rs#target_name: a name already carrying the prefix keeps its number, and zero-padding may now narrow as well as widen while the value is unchanged; the error message no longer says NNNN. The claim that the target follows the declared pattern still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
