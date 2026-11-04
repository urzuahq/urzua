---
Stable-Id: 01M6JKNNNS5KTTBK4QAR0D3DEC
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/embodiment.rs, test:rust/crates/urzua-cli/tests/sticky_state_integration.rs
---
# 148 — Stated Deprecated and Inactive Embodiment are sticky human-set states with no computed counterpart

## What

Let `embodiment.consistency` treat a stated `Deprecated` or `Inactive` Embodiment as a sticky state a human sets, not a tier that evidence must reproduce, so it is not reported as drift.

## Why

A real adopter corpus has dozens of records stating `Deprecated` or `Inactive`. Nothing can be computed for them, so `embodiment.consistency` reports every one as drift. A finding the author cannot resolve by changing the artifact trains readers to ignore the rule. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A record stating `Deprecated` or `Inactive` produces no consistency finding on that account; the set of sticky states is declared in configuration; a fixture covers a sticky state and a genuine drift on a neighbour; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Sticky Embodiment states declared in configuration (`SPEC-2` `0.36`, `SPEC-3` `0.22`): this repository declares `sticky_states: [Deprecated, Inactive]` and `ADR-29`, superseded by `ADR-45`, now states `Deprecated`; a fixture covers sticky states beside a genuine drift. **Why:** the Done-when asks for it. Status unchanged; the maintainer closes. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #199: declared `sticky_states` are not drift and `fix` leaves them alone. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
