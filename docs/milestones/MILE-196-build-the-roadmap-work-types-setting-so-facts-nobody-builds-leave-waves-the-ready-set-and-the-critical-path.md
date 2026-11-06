---
Stable-Id: 01M6NVCAB8J6WF7579JXM1PPYF
Status: Done
Realized-by: code:rust/crates/urzua-core/src/rules/roadmap.rs, code:rust/crates/urzua-core/src/config.rs, code:.urzua/config.yaml, spec:docs/specs/SPEC-3-configuration.md, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-23-the-condition-record-type.md, test:rust/crates/urzua-cli/tests/work_types_integration.rs
Phase: '0'
Track: roadmap-tracking
Implements: ADR-82
Blocked-on: null
---
# 196 — Build the roadmap work_types setting so facts nobody builds leave waves, the ready set and the critical path

## What

The `roadmap.work_types` setting of `ADR-82`'s amendment of 2026-11-05: an optional list of record type names that are work. With it declared, `urzua roadmap` keeps a non-work record out of `waves`, `ready`, `critical_path`, `records` and `groups`, still lets it block its dependents, and lists it in a top-level `waiting_on`. With it absent the output is unchanged.

## Why

Since `MILE-187` (#349) the open `condition` records `COND-1` to `COND-4` are in wave 0 and the ready set, and `COND-1` is on the critical path, as if they were buildable. A condition is met or dropped, never built, and no configuration can say so today (the review of #349, point 2). The maintainer decided on 2026-11-05 to amend `ADR-82` with an opt-in setting. This milestone builds it once the maintainer accepts the amendment, so it is blocked on that acceptance.

## Done means

- `roadmap.work_types` loads as a list of `record_types` keys. An unknown name, a duplicate, a blank name and an empty list are each a `roadmap: ...` load error naming the value, with tests that fail before and pass after.
- With the key absent, `urzua roadmap` and `export --format json` are byte-identical to before on a fixture corpus and on this repository (a test compares them); no `waiting_on` key.
- With the key declared, in a fixture whose non-work type is not named `condition`: a non-work record is in no wave, not `ready`, not on `critical_path`, not a key of `records`, not in `groups`, not counted by `nothing-open` or `unstructured-wait`; a dependent of an `Open` one is `blocked` and not `ready`, of a `Met` one released, of a `Dropped` one in `needs_replan` as `failed-dependency` via the non-work record; `blocks` of a work record reaches work records through a non-work one; waves and the critical path count work records only, with edges contracted through chains of non-work ones (a test pins the contracted chain: open work `W`, non-work `C` depending on `W`, work `R` depending on `C`, where `R` is in a later wave than `W`, and `W`, `R` is a critical-path chain; `C` `Open` still blocks `R`, `Met` releases it, `Dropped` sends it to `needs_replan`); `ready` is a subset of wave 0. `waiting_on` lists each non-work record not in `success` with `type`, `status` and `blocks`, in natural id order.
- `sequence.cycle` (including the cross-pair reading of `MILE-192`) and `sequence.status-agreement` give the same findings with and without `work_types` on a corpus that has a loop through a non-work record and a work record blocked by an `Open` non-work one (a test pins both).
- `SPEC-2` (roadmap output), `SPEC-3` (the key and its load errors) and `SPEC-23` (how a condition is read) describe it with revision rows, and a changeset adds it (new capability: minor).
- This repository's `.urzua/config.yaml` declares `work_types` listing every record type except `condition`, and `urzua roadmap` then shows `COND-1` to `COND-4` in none of `waves`, `ready` and `critical_path`, with `MILE-13`, `MILE-21`, `MILE-23` and `MILE-30` still `blocked` by them and listed under `waiting_on`. `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** Maintainer decision, 2026-11-05: amend `ADR-82` with an opt-in `roadmap.work_types` so facts nobody builds leave the ready set; the amendment is proposed and this milestone waits for the maintainer to accept it. | **substantive** |
> | 2026-11-05 | Done means now requires a test for the contracted chain through a non-work record, and that `ready` is a subset of wave 0 (`SPEC-2` says so). **Why:** review of `#350`: waves must agree with `blocks`. | **substantive** |
> | 2026-11-05 | `Status` Planned -> Done; `Realized-by` added. **Why:** Done means is met and the maintainer accepted the `ADR-82` Amendment (`#365`). `roadmap.work_types` loads (an unknown, repeated, blank or padded name and an empty list are each a `roadmap: ...` error); absent, `roadmap` and `export --format json` are byte-identical to before on this repository and a fixture corpus and carry no `waiting_on`; declared, a non-work record gates but is in no wave, `ready`, `critical_path`, `records`, `groups` or `needs_replan`, waves and the critical path contract through non-work chains (the worked example is a test, for `Open`, `Met` and `Dropped`), `ready` is a subset of wave 0, and `waiting_on` lists the unfinished non-work records; `sequence.cycle` and `sequence.status-agreement` give the same findings with and without it. `SPEC-2`, `SPEC-3` and `SPEC-23` have revision rows, and this repository lists every type but `condition`, so `COND-1` to `COND-4` are in no wave, not ready and off the critical path while `MILE-13`, `MILE-21`, `MILE-23` and `MILE-30` stay blocked and listed under `waiting_on`. | **substantive** |
