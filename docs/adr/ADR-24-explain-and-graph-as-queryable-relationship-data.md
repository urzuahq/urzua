---
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/graph.rs@sha256:c54bd827556265c9e1132b1c469424a71d73228c053c76099b4fd8f881e5ea67, code:rust/crates/urzua-cli/src/commands/explain.rs@sha256:b8bc8e59f92624247bca1d5c8bf899ed6038569b0293d743f63bbdd0a323c445, code:rust/crates/urzua-cli/src/commands/graph.rs@sha256:41be294e4e7cf37f0df56bc57456c97a40d9332a2de7f001ab98254d87326c2b, test:rust/crates/urzua-core/src/graph.rs@sha256:c54bd827556265c9e1132b1c469424a71d73228c053c76099b4fd8f881e5ea67
Date: 2026-09-06
Accepted-on: 2026-09-06
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
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 46535c832. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/graph.rs` 1638485fe98c -> 33b1c278a9ff, `rust/crates/urzua-cli/src/commands/explain.rs` b08e0b32fa62 -> 42e8ee239a73, `rust/crates/urzua-cli/src/commands/graph.rs` 86de33bf27bc -> 41be294e4e7c. Reason: MILE-173 moved the shared root, config, discovery and record-load preamble of explain and graph into session.rs; their output and exit codes are unchanged, pinned by could_not_run_integration.rs **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/graph.rs` 33b1c278a9ff -> 4ac57e6f2018, `rust/crates/urzua-cli/src/commands/explain.rs` 42e8ee239a73 -> b8bc8e59f926. Reason: Drift review of 2026-11-04, recommendation (a). explain still answers which records name a path as evidence, by exact path and with the matching category, and graph still emits the same edges. Since the pin, explain gained path#scope (ADR-74): graph.rs reads Realized-by through the shared locator parser, which parse_realized_by itself now wraps, and not through parse_realized_by directly, so this decision's wording 'reuse parse_realized_by' is met in substance only; governing records also carry an optional scope. explain.rs resolves a scope and prints its span and pin. Graph edge building is untouched. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/graph.rs` 4ac57e6f2018 -> c54bd8275562. Reason: BUG-74 renamed Waiver to Suppression (and load_waivers to load_suppressions) in this file's internals; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
