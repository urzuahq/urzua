# SPEC-13 — `urzua explain` and `urzua graph`

> Version: 0.1 | Date: 2026-09-07 | Status: Accepted
> **Implements:** ADR-24
> **Parent:** SPEC-1 (v0 CLI).

## Purpose

Two read-only relationship queries over data every other rule already parses — neither needs a new
schema field or config change. Bundled into one spec because they're the same feature area (queryable
relationship data) answering two directions of the same question: "which decisions govern this
file" (`explain`) and "what does the whole graph look like" (`graph`).

## `urzua explain <path>`

Every record whose `Realized-by` names `path` as evidence, with which category (`spec:`/`code:`/
`test:`) matched. Reuses ADR-18's Embodiment MVP's categorized-locator data directly — no new field.

Exact-match only: no glob, no directory prefix, no fuzzy match. A corpus citing a file by a
different relative path than the one queried finds nothing. Coverage is only as complete as
`Realized-by` adoption — a record that genuinely governs a file but never had a `Realized-by` field
added is invisible to `explain`, the same limitation `embodiment.consistency` already has.

## `urzua graph`

Every `Implements`/`Derives-from`/`Supersedes`/`Superseded-by`/`Parent` edge across the whole corpus,
each tagged `dangling: bool` — the same condition `pointer.resolution` computes, exposed as an edge
property instead of requiring a separate `check` run to notice. No filtering (`--type`, `--format
dot`) — dumps every edge.

## Shared implementation

Both reuse `record_id`/`extract_references`/`parse_realized_by` (`rules.rs`) rather than
reimplementing reference-parsing a third time. Both are stdout-JSON-always (ADR-23), require no
config change, and never write.

## What's deliberately not built

- **`explain` fuzzy/prefix matching** on `Realized-by` locators — real follow-up, not attempted.
- **`graph --type`/`--format dot` filtering** — additive later work once a real case needs a
  narrower or differently-shaped view than "every edge."

## References

- ADR-24 — the decision this spec details: both commands, the data they read, why neither needs a
  schema change.
- ADR-18 — the categorized-locator model `explain` reads.
- ADR-23 — the stdout-JSON-always contract both commands follow.
- `rust/crates/urzua-core/src/rules.rs` — `record_id`, `extract_references`, reused by both.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, bundling `explain` and `graph` since they're one feature area (queryable relationship data) answering two directions of the same question. **Why:** MILE-77 found both documented only as a single ADR while comparable-complexity command areas (`check`, `init`) had specs. | **structural** |
