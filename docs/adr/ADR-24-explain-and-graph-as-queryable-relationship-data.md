---
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/graph.rs@sha256:1638485fe98c800c7494d800ea27219c2df5afe525df75d7a0688a66947b702b, code:rust/crates/urzua-cli/src/commands/explain.rs@sha256:b08e0b32fa62fec151a02bebcc6761706881e0fe2b928007d2afc5e64c08682e, code:rust/crates/urzua-cli/src/commands/graph.rs@sha256:86de33bf27bc82f3da6783c3d2840c1937d82f6a420586f9d34dabfca28dc851, test:rust/crates/urzua-core/src/graph.rs@sha256:1638485fe98c800c7494d800ea27219c2df5afe525df75d7a0688a66947b702b
Date: 2026-09-06
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-1, ADR-18
---
# 24 — `urzua explain` and `urzua graph`: the relationship graph as queryable data

## Context

Two named, unimplemented gaps: no command answers "which decisions govern this file" (research
flagged this as the single highest-value read command in the landscape), and no command exposes the
relationship graph (`Implements`/`Derives-from`/`Supersedes`) as queryable data — a reader has to
run `check` and eyeball warnings to reconstruct it. Both are answerable entirely from data every
existing rule already parses; neither needs a new schema field.

## Decision

In the context of a fully agent-driven tool, facing two real, valuable read paths with no command
surface, we decided on two new commands, both stdout-JSON-always (ADR-23):

**`urzua explain <path>`** — every record whose `Realized-by` names `path` as evidence, with which
category (`spec`/`code`/`test`) matched. Answers "which decisions govern this file" from the same
categorized-locator data ADR-18's Embodiment MVP already introduced — no new field.

**`urzua graph`** — every `Implements`/`Derives-from`/`Supersedes`/`Superseded-by` edge across the
whole corpus, each tagged `dangling: bool` (the same condition `pointer.resolution` and
`relation.supersession-reciprocity` already compute, exposed as an edge property instead of
requiring a separate `check` run to notice).

Both are read-only, reuse `record_id`/`extract_references`/`parse_realized_by` rather than
reimplementing reference-parsing a third time, and require no config change or new record field.

## Reversibility

Fully additive: two new commands over existing data. Neither changes what `check`/`fix` compute or
how any existing field is parsed.

## Consequences

- `explain` only ever matches an exact locator string against `Realized-by` — no glob, no directory
  prefix, no fuzzy match. A corpus citing a file by a different relative path than the one queried
  finds nothing; broadening this is real follow-up work, not attempted here.
- `graph` has no filtering (`--type`, `--format dot`) yet — it dumps every edge. Scoping it down is
  additive later work.
- `explain`'s coverage is only as complete as `Realized-by` adoption — a record that governs a file
  but has never had a `Realized-by` field added is invisible to it, same limitation
  `embodiment.consistency` already has.

## References

- ADR-18 — the categorized-locator model `explain` reads.
- `rust/crates/urzua-core/src/rules.rs` — `record_id`, `extract_references`, reused rather than
  reimplemented.
- ADR-23 — the stdout-JSON-always contract both commands follow.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
