---
Status: Done
Stable-Id: 01M1YNND3FMY2F2A0N3K0FVT3H
Phase: '0'
Track: governance-process
Implements: ADR-24
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#explain_names_the_governing_record_and_graph_reports_kind_and_dangling_per_edge@sha256:2dfa7fba6ef52a989d113ad2a97aa02380d6bf8e4324f952ce3704767195d15d, code:rust/crates/urzua-cli/src/commands/explain.rs, code:rust/crates/urzua-cli/src/commands/graph.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing@sha256:1fa8afd36075b9642d82262683c6cbd82cffa2c548edb7dc234e4be05dea405c, spec:docs/specs/SPEC-13-urzua-explain-and-urzua-graph.md
---
# 63 — Build urzua explain and urzua graph

## What

`urzua explain <path>`: every record whose `Realized-by` names that file as evidence. `urzua graph`: the full `Implements`/`Derives-from`/`Supersedes` relationship graph as data, flagging edges that don't resolve.

## Why

The record graph existed in the schema from the start but nothing surfaced it as queryable data -- `explain` answers "which decisions govern this file," the highest-frequency read an agent or reviewer actually has.

## Done means

- `urzua explain <path>` lists every record whose `Realized-by` names that file. — evidence: test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#explain_names_the_governing_record_and_graph_reports_kind_and_dangling_per_edge@sha256:2dfa7fba6ef52a989d113ad2a97aa02380d6bf8e4324f952ce3704767195d15d, code:rust/crates/urzua-cli/src/commands/explain.rs
- `urzua graph` emits the `Implements`/`Derives-from`/`Supersedes` graph as data with unresolved edges flagged. — evidence: test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#explain_names_the_governing_record_and_graph_reports_kind_and_dangling_per_edge@sha256:2dfa7fba6ef52a989d113ad2a97aa02380d6bf8e4324f952ce3704767195d15d, code:rust/crates/urzua-cli/src/commands/graph.rs
- Both `explain` and `graph` are read-only. — evidence: code:rust/crates/urzua-cli/src/commands/explain.rs, code:rust/crates/urzua-cli/src/commands/graph.rs
- Both are covered in `check_integration.rs`. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing@sha256:1fa8afd36075b9642d82262683c6cbd82cffa2c548edb7dc234e4be05dea405c
- Both are documented in `SPEC-13`. — evidence: spec:docs/specs/SPEC-13-urzua-explain-and-urzua-graph.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
