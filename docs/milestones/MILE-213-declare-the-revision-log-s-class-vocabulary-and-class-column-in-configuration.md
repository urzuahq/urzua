---
Stable-Id: 01M6SVM9AAVXSGR3VDCQB18KH4
Status: Done
Phase: '0'
Track: schema-governance
Implements: ADR-94
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/rules/revision_log.rs, code:rust/crates/urzua-core/src/fix.rs, code:rust/crates/urzua-cli/src/commands/fix.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-w/expectations/a-declared-vocabulary-and-column-are-read-from-the-header.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-w/expectations/only-the-declared-bump-classes-require-a-bump.yaml, test:rust/crates/urzua-cli/tests/fix_revision_log_required_integration.rs, spec:docs/specs/SPEC-3-configuration.md, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-4-the-corpus-acceptance-suite.md, spec:docs/specs/SPEC-8-urzua-fix.md
---
# 213 — Declare the revision log's class vocabulary and class column in configuration

## What

Build `ADR-94`: a record type declares the revision log's `classes`, its `class_column` and optional `date_column` (header
names) and the `tool_class` that `fix` writes, as optional keys of `revision_log`; `revision-log.change-class-required`,
`revision-log.version-bump-required` and `urzua fix` read them, and a type declaring none behaves exactly as today.

## Why

`ADR-64` makes config-driven the whole thesis, yet the rule names `substantive` and `structural` and reads the
class from the last cell (`rust/crates/urzua-core/src/rules/revision_log.rs:62`, `:293`), and `fix` writes
`**structural**` (`rust/crates/urzua-core/src/fix.rs:385`, `:632`). Maintainer decision of 2026-11-07: the rule must
not hardcode its vocabulary, and Phase 0 is capability parity exercised in urzua's own repository.

## Done means

- A type declaring `classes` accepts exactly those classes, and the finding lists them.
- A type declaring no `classes` reports the message it reports today, and every existing fixture and acceptance corpus passes unchanged.
- A declared `class_column` or `date_column` selects the cell by header name (exact match after stripping emphasis and whitespace, case-sensitive), in any position; undeclared, the class is the last cell and the date the first.
- A log whose header lacks a declared column, or has it more than once, is one finding on the record carrying an `edit-section` `suggested_action` (with an `action_source` and a `SPEC-2` row).
- `revision-log.version-bump-required` reports a record whose log it cannot read as unreadable, not clean; so does a base log it cannot read, which is never treated as empty.
- `date_column` and `class_column` naming the same header, and `classes` entries equal after stripping, fail config load.
- A `version_bump.classes` entry outside a declared `classes` fails config load naming the entry; with `classes` undeclared it is not checked. A declared class not in `version_bump.classes` requires no bump.
- `fix --apply` and `fix --rebaseline` write the type's `tool_class`, laid out by the log's header (date column, class column, narrative in the first remaining column).
- A `tool_class` outside `classes`, or a vocabulary without `structural` and without `tool_class`, fails config load.
- `.urzua/config.yaml` declares today's values (`classes`, `class_column`, `date_column`, `tool_class`) on all seven types.
- `urzua check` and `urzua check --base origin/main` report 0 unwaived findings.
- An acceptance corpus with another vocabulary and a reordered table (`Class | Date | Change`) exercises the declared path.
- `SPEC-2`, `SPEC-3`, `SPEC-4` and `SPEC-8` carry the change, with no `schema_version` bump.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed. **Why:** maintainer decision of 2026-11-07 (`ADR-94`, Proposed): the revision-log rule must not hardcode its class vocabulary or its class column; this repository declares today's values and an undeclared config keeps today's behaviour. | **substantive** |
> | 2026-11-07 | Done means updated for the accepted `ADR-94` review changes: optional `date_column`, exact header matching, duplicate header cells, an unreadable base log is unreadable, the new finding's `suggested_action`. **Why:** maintainer decision, 2026-11-07, accepting `ADR-94` with the review's changes (`#391`). | **substantive** |
> | 2026-11-07 | Done: built as `ADR-94` decides. Realized-by names the config keys and their load errors, the rule and parser, the fix writer, the `corpus-w` acceptance corpus and the specs; this repository declares today's values on all seven types. **Why:** every Done-means line is delivered and exercised, in the unit tests, the integration test and `corpus-w`, and by `urzua check` and `check --base` on this repository. | **substantive** |
> | 2026-11-07 | Independent review: a row wider than its header is no longer read by a last-column guess, which misread any declared class column that was not last. A `\|` now stays inside its cell, and under a declared column a wider row is one finding naming it; six unescaped pipes in existing revision-log rows are escaped. **Why:** the guess could pass or fail a row by luck, and a finding that names the cause is more honest than a parse that hides it. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
