---
Stable-Id: 01M6K40SNCNT3C891H8MNEMD1J
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-core/src/rules.rs
---
# 175 — Test fixtures build RecordTypeConfig through Default

## What

Build test fixtures for `RecordTypeConfig` through `Default`: derive `Default` on `RecordTypeConfig` where the field types allow it (`HeaderShape` already has one) and use `..Default::default()` in `type_config`, `type_config_with_shape` and `type_config_pointer` in `urzua-core/src/rules.rs`, and in the hand-built `Config` literal. Fix the stale "The two seed rules" header comment in that file.

## Why

`RecordTypeConfig` has no `Default`, so each of those three fixtures spells out all 14 fields and every new field edits every fixture. That is cost on every future option, and it hides which field a test actually cares about. It is the smallest piece of the foundation set and can land at any time. Evidence read at the foundation review of 2026-11-04.

## Done means

The three fixtures name only the fields a test varies; adding a field to `RecordTypeConfig` does not touch a fixture; the stale comment is corrected; `cargo test --workspace` is unchanged in result.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
> | 2026-11-04 | `Status` Planned -> Done, `Realized-by` added. `RecordTypeConfig` and `Config` derive `Default`; `type_config`, `type_config_with_shape`, `type_config_pointer` and `config_with_types` name only the fields a test varies. **Why:** checked against Done means: the fixtures no longer list all 14 fields, and `cargo test --workspace` is unchanged. The stale "two seed rules" comment was already gone when this was built (the file was reorganised by `MILE-171`), so nothing was left to correct. | **structural** |
