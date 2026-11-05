---
Stable-Id: 01M6K53B7R9MSAHEPCE46XJ91G
Status: Done
Realized-by: code:rust/crates/urzua-core/src/rules/roadmap.rs, code:rust/crates/urzua-cli/src/commands/roadmap.rs, test:rust/crates/urzua-cli/tests/roadmap_integration.rs
Phase: '0'
Track: roadmap-tracking
Implements: RFC-48
Depends-on: MILE-27, MILE-144, MILE-115, ADR-82
Blocked-on: —
---
# 182 — urzua roadmap computes waves, the ready set and the critical path from records

## What

A read-only `urzua roadmap` command that computes the order of work from the records alone, as `RFC-48` section 2 proposes. For a scope (by default every type that declares an acyclic sequencing relation) it emits JSON holding the waves (topological layers over open records), the ready set (open records whose blockers are all in a success terminal state), the critical path (the longest chain of open blockers), what each record blocks transitively, and a grouping by the fields the configuration names. The same data is part of `export --format json` (`MILE-115`). Nothing is stored. It is a generic capability: it reads declared relations and declared status vocabularies and holds no knowledge of any record type, issue tracker or product.

## Why

`RFC-48` is accepted. The order of work in a corpus is real information that `Phase` and `Track` cannot express, and today it lives in a hand-built page that nothing checks against the records. A corpus of this size can list findings but cannot say what can start now. A real adopter environment keeps a roadmap alongside its records, and Phase 0 is urzua reproducing that functionality in its own repository: the computed roadmap is part of the Phase 0 tooling by maintainer decision of 2026-11-04.

It depends on the relation and the derived value other milestones build, and does not duplicate them. `MILE-27` declares the sequencing relation (`Depends-on`/`Blocks`) and its cycle check. `MILE-144` supplies the derived `blocked` value. `MILE-135` is the later, separate tool that adds open changes on a forge as progress evidence; it needs this command first and is out of scope here, as `RFC-48` excludes forge input.

## Done means

`urzua roadmap` on a fixture corpus prints waves, a ready set and a critical path that match a hand-computed answer, including a record blocked on a record in a failure terminal state (kept out of the ready set), a cycle (reported, never looped on) and a declared relation that matches no record (reported, not a silent empty result). A repository that declares no acyclic pair gets an explicit statement that there is no roadmap, not an empty one. Grouping fields come from configuration, not from the engine. The output shape is fixed in a `SPEC` revision and appears in `export --format json`.

In this repository, `urzua roadmap` run against `.urzua/config.yaml` and the committed records generates urzua's own roadmap: its ready set and critical path are read from the records' `Depends-on` values, and the hand-built Phase 0 roadmap page, where one exists, is retired in favour of a page generated from that output.

## References

- `RFC-48` -- section 2, `urzua roadmap`; section 4, this repository's workflow.
- `MILE-27`, `MILE-144` -- the sequencing relation and the derived blocked value this reads.
- `MILE-135` -- later: open changes as progress evidence.
- `MILE-115` -- `export --format json`, the data feed.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** Maintainer decision, 2026-11-04: the computed roadmap (`RFC-48`, Accepted) is part of the Phase 0 tooling, so urzua generates its own roadmap from its records. | **substantive** |
> | 2026-11-04 | `Depends-on: MILE-27, MILE-144` added; `MILE-27` removed from `Blocked-on`. **Why:** `MILE-27` is built, so the sequencing relation exists and this record is the first to carry it; `Blocked-on` keeps the note on `MILE-144`, which is still open. `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-115` added. **Why:** Done means puts the roadmap data in `export --format json`, the feed `MILE-115` built (Done). Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: ADR-82` added and `Blocked-on` set to it; `Status` stays `Planned`. **Why:** `RFC-48` leaves the roadmap's waves, ready set, critical path and output shape open and no accepted record decides them; building part of an RFC needs its own ADR. `ADR-82` is drafted as Proposed and the maintainer decides it; nothing of `urzua roadmap` is built until it is Accepted. `blocked` is derived from `Depends-on` and is not written, so no status change is needed. | **substantive** |
> | 2026-11-05 | `Blocked-on` cleared. **Why:** ADR-82 is Accepted; the dependency stays declared in `Depends-on`. | **structural** |
> | 2026-11-05 | `Status` Planned -> Done; `Realized-by` added. **Why:** Done means is met. `urzua roadmap` prints waves, a ready set and a critical path that match hand-computed answers (`roadmap_integration.rs`, and the acceptance cases of `corpus-k`) for a record blocked on a failure-terminal record (kept out of every list, with a `needs_replan` reason), a cycle (reported once, never looped on), a declared pair that matches no record and a claim that resolves to nothing (both notes), a loop across two declared pairs (a cycle), and the critical-path tie-break; a repository that declares no `roadmap.pairs` gets a `no-sequencing-pair` note, not an empty roadmap. Grouping comes from `roadmap.group_by`, validated at load, and the engine names no field. The output shape is in `SPEC-2` and `SPEC-3` and appears in `export --format json` when `roadmap.pairs` is declared. This repository declares `roadmap.pairs` and `group_by` in `.urzua/config.yaml`, and `make roadmap` prints its own roadmap, with its ready set and critical path read from `Depends-on` values. No hand-built Phase 0 roadmap page is committed in this repository, so there is none to retire. Built to `ADR-82`. | **substantive** |
