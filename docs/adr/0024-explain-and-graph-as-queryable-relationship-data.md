# 0024 — `urzua explain` and `urzua graph`: the relationship graph as queryable data

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-core/src/graph.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/graph.rs
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0001 (Accepted), ADR-0018 (Accepted)

## Context

Two named, unimplemented gaps: no command answers "which decisions govern this file" (research
flagged this as the single highest-value read command in the landscape), and no command exposes the
relationship graph (`Implements`/`Derives-from`/`Supersedes`) as queryable data — a reader has to
run `check` and eyeball warnings to reconstruct it. Both are answerable entirely from data every
existing rule already parses; neither needs a new schema field.

## Decision

In the context of a fully agent-driven tool, facing two real, valuable read paths with no command
surface, we decided on two new commands, both stdout-JSON-always (ADR-0023):

**`urzua explain <path>`** — every record whose `Realized-by` names `path` as evidence, with which
category (`spec`/`code`/`test`) matched. Answers "which decisions govern this file" from the same
categorized-locator data ADR-0018's Embodiment MVP already introduced — no new field.

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

- ADR-0018 — the categorized-locator model `explain` reads.
- `rust/crates/urzua-core/src/rules.rs` — `record_id`, `extract_references`, reused rather than
  reimplemented.
- ADR-0023 — the stdout-JSON-always contract both commands follow.
