---
Stable-Id: 01M6CXZET544KZ3KCXNVXP39XS
Status: Done
Realized-by: code:rust/crates/urzua-cli/src/commands/export.rs, code:rust/crates/urzua-core/src/rules/section.rs, test:rust/crates/urzua-cli/tests/export_integration.rs
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

## Done means

`urzua export --format json` emits every record of every configured type as plain JSON data (display id, `Stable-Id`, type, path, header fields and raw text), the field names are in `SPEC-1`'s output contract, `scripts/generate-dashboard.py` reads records only through it and no longer globs or parses `docs/`, and the record's sections are emitted for each type that declares where sections come from. Each record carries `sections` and `preamble`, `null` for a type that declares none.

## References

- `SPEC-1` -- the one-config-driven-implementation constraint a second parser violates.
- `MILE-102` -- the first-adopter replacement this Phase 0 gap work serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `export` declared only `agdr` and was unimplemented, so the dashboard script grew a second parser that already produced a wrong published count; filed so tooling reads records through `urzua`. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #137: `urzua export --format json` emits every record of every configured type as plain JSON. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Reopened: `Status` Done -> InProgress; `Realized-by` removed, evidence moved here. **Why:** the What asks for the records' sections to be exported once the declared document model says where sections come from. That condition now holds (the ADR and bug types declare `sections` in `.urzua/config.yaml`), and `urzua export --format json` emits only `type`, `path`, `id`, `stable_id`, `header_readable`, `header` and `raw_text`, no sections, so the What is not fully met. The plain-JSON export and the dashboard switch are delivered. Evidence kept from the Done state, which was a partial delivery and not the milestone's outcome: `code:rust/crates/urzua-cli/src/commands/export.rs, test:rust/crates/urzua-cli/tests/export_integration.rs`. A `Realized-by` under a not-done `Status` is a disagreement `status.delivery-agreement` (`MILE-183`) reports, so it is recorded in this row, as in `MILE-128`. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those this record's What asks for, and they are not met (see the previous row); `Status` is InProgress. | **structural** |
> | 2026-11-04 | `Status` InProgress -> Done; `Realized-by` added. **Why:** `export` now emits `sections` (name, heading, line span, content) and `preamble` for each type that declares `sections`, through the parser `section.required` uses, with aliases resolved to the declared name (`MILE-165`); a type declaring none gets an explicit `null`, not an absent key, so no-declaration is distinct from declared-and-empty. Named in `SPEC-1`'s export contract; `scripts/generate-dashboard.py` reads the milestone What and Why from `sections` and its output is unchanged. | **substantive** |
