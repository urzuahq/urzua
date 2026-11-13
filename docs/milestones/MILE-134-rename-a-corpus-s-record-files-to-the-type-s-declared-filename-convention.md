---
Stable-Id: 01M6DEN9P605CKC9GV4VVTE0PV
Status: Planned
Phase: '1'
Track: write-path
---
# 134 — Rename a corpus's record files to the type's declared filename convention

## What

`urzua migrate filenames`: rename a corpus's record files to the filename convention its type's `identity.pattern` declares (`TYPE-NNNN-slug.md` for this repository's `ADR-36` corpus), and rewrite relative links and path mentions to the renamed files, reporting every rewrite. Dry run by default.

## Why

`RFC-46` was rejected in favour of one convention: a corpus in another shape is renamed rather than tolerated. The first such adoption is renamed by a one-off script inside its own switchover change, so the tool is later work; it becomes worth building once adopting a second corpus repeats the same rename.

## Done means

- `urzua migrate filenames` renames a corpus's record files to the convention the type's `identity.pattern` declares, never a hardcoded shape. — evidence: code:rust/crates/urzua-core/src/migrate_filenames.rs#target_name, test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#the_target_follows_the_declared_pattern_never_a_hardcoded_shape
- It rewrites relative links and path mentions to the renamed files and reports every rewrite. — evidence: code:rust/crates/urzua-core/src/migrate_filenames.rs#rewrite_citations, test:rust/crates/urzua-core/src/migrate_filenames.rs#citations_resolve_against_the_file_or_the_root_and_nothing_else, test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#apply_renames_keeps_identity_rewrites_citations_and_check_passes
- It is a dry run by default. — evidence: test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#a_dry_run_lists_every_rename_and_citation_and_writes_nothing
- A second applied run changes nothing. — evidence: test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#a_second_applied_run_changes_nothing
- A fixture with bare zero-padded filenames renames to `TYPE-NNNN-slug.md` and then passes `check`. — evidence: test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs#apply_renames_keeps_identity_rewrites_citations_and_check_passes
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
