---
Version: '0.4'
Date: 2026-09-07
Status: Accepted
Author: '@beauwilliams'
Subject: '`urzua explain`/`urzua graph` -- read-only relationship queries over already-parsed data.'
Implements: ADR-24, ADR-44
Parent: SPEC-1
---
# SPEC-13 — `urzua explain` and `urzua graph`

## Purpose

Two read-only relationship queries over data every other rule already parses — neither needs a new
schema field. `explain` needs no config change either; `graph` reads `pointer_fields`/
`narrative_fields` from existing config (MILE-90/ADR-44), not a new key. Bundled into one spec
because they're the same feature area (queryable relationship data) answering two directions of the
same question: "which decisions govern this file" (`explain`) and "what does the whole graph look
like" (`graph`).

## `urzua explain <path>`

Every record whose `Realized-by` names `path` as evidence, with which category (`spec:`/`code:`/
`test:`) matched. Reuses ADR-18's Embodiment MVP's categorized-locator data directly — no new field.

Exact-match only: no glob, no directory prefix, no fuzzy match. A corpus citing a file by a
different relative path than the one queried finds nothing. Coverage is only as complete as
`Realized-by` adoption — a record that genuinely governs a file but never had a `Realized-by` field
added is invisible to `explain`, the same limitation `embodiment.consistency` already has.

## `urzua graph`

Every edge named by a type's config-declared `pointer_fields` ∪ `narrative_fields` (MILE-90/ADR-44),
plus the still-hardcoded `Supersedes`/`Superseded-by` (its own mechanism, outside that axis), across
every record with a resolvable identity (`record_id`) — a record whose filename doesn't match its
type's configured prefix/number shape contributes no outgoing edges, the same precondition
`pointer_resolution`'s index-building already has. Config-driven, not a hardcoded field list — a
type's own `Parent`, or any other declared relationship field, appears for free the moment it's
declared, with no code change here.

Each edge is tagged:

- `kind: "pointer" | "narrative"` — the field's declared kind (MILE-90/ADR-44). Describes the edge's
  value shape (clean reference vs. prose-tolerant), not which rule resolved it: `Supersedes`/
  `Superseded-by` reports `pointer`, since its values are always clean references, even though
  `relation.supersession-reciprocity` (a separate mechanism) is what actually checks it.
- `dangling: bool` — the same condition `pointer.resolution` computes, exposed as an edge property
  instead of requiring a separate `check` run to notice.

No filtering (`--type`, `--format dot`) — dumps every edge.

## Shared implementation

Both reuse `record_id`/`extract_references`/`parse_realized_by` (`rules.rs`) rather than
reimplementing reference-parsing a third time; `graph` additionally reuses `build_normalized_index`,
the same normalized-lookup helper `pointer_resolution`/`narrative_field_stale` use (MILE-90 closed
BUG-11 here: this function's own index used to skip normalization, unlike `pointer_resolution`'s).
Both are stdout-JSON-always (ADR-23) and never write. `graph` reads `pointer_fields`/
`narrative_fields` from config (MILE-90/ADR-44); `explain` still requires no config change.

## What's deliberately not built

- **`explain` fuzzy/prefix matching** on `Realized-by` locators — real follow-up, not attempted.
- **`graph --type`/`--format dot` filtering** — additive later work once a real case needs a
  narrower or differently-shaped view than "every edge."

## References

- ADR-24 — the decision this spec details: both commands, the data they read, why neither needs a
  schema change.
- ADR-44 — `pointer_fields`/`narrative_fields` as config-declared per type (MILE-90); `graph`'s
  edge set and `kind` field are this decision's realization.
- ADR-18 — the categorized-locator model `explain` reads.
- ADR-23 — the stdout-JSON-always contract both commands follow.
- `rust/crates/urzua-core/src/rules.rs` — `record_id`, `extract_references`, `build_normalized_index`,
  reused by both/`graph`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, bundling `explain` and `graph` since they're one feature area (queryable relationship data) answering two directions of the same question. **Why:** MILE-77 found both documented only as a single ADR while comparable-complexity command areas (`check`, `init`) had specs. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-09 | `urzua graph` is now config-driven (MILE-90/ADR-44): edges come from each type's declared `pointer_fields`/`narrative_fields`, not a hardcoded field list, and every edge gains a `kind` field. Fixed BUG-11 in the same pass (`graph`'s own index never normalized reference padding, unlike `pointer_resolution`'s). Added `Implements: ADR-44`. **Why:** this spec's own text previously claimed `graph` "requires no config change," which stopped being true the moment this shipped -- leaving it unstated would have been the exact "spec goes factually stale the moment a decision lands" gap this project's own history keeps flagging. | **substantive** |
