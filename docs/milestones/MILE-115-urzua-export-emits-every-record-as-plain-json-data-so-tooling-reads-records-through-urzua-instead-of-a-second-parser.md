---
Stable-Id: 01M6CXZET544KZ3KCXNVXP39XS
Status: Planned
Phase: '0'
Track: roadmap-tracking
---
# 115 — urzua export emits every record as plain JSON data, so tooling reads records through urzua instead of a second parser

## What

`urzua export` is not implemented today: `--format agdr` is the only declared value and the command
exits `not implemented`. Add a plain-JSON export that emits every record of every configured type as
data: its identity (display id and `Stable-Id`), type, path, and header fields, plus its sections once
`MILE-98`'s document model declares where sections come from (no type declares sections today). The
exact JSON field names are part of this milestone's deliverable and belong in `SPEC-1`'s output
contract. AgDR export stays `MILE-21`'s scope. Then switch `scripts/generate-dashboard.py` to consume that output instead
of globbing `docs/` and parsing it with its own Python and YAML logic.

## Why

`scripts/generate-dashboard.py` is a second parser of the corpus, written in a different language with
different rules about what a header is, so it can disagree with `check` about what the corpus contains.
It has: the published dashboard drifted to showing 3 bugs while the corpus held roughly 145. Tooling
that reads records should read them through `urzua`, the one implementation `check` itself uses, so a
parsing fix lands everywhere at once. This is the same one-implementation argument `SPEC-1` makes
against every team hand-writing its own linter, applied to this repository's own tooling.

## References

- `SPEC-1` -- the one-config-driven-implementation constraint a second parser violates.
- `MILE-102` -- the first-adopter replacement this Phase 0 gap work serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `export` could emit only `agdr`, so the dashboard script grew a second parser that already produced a wrong published count; filed so tooling reads records through `urzua`. | **substantive** |
