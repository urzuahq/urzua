---
Stable-Id: 01M6JKNPGW97WYMKFRJ94935W6
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 149 — Embodiment.consistency judges a record with no Realized-by field when back-pointers cite it

## What

Judge a record that has no `Realized-by` field when code back-pointers cite it, instead of skipping it.

## Why

The rule does not judge such a record, so the adopter would have to add an explicit empty `Realized-by` to every record in a corpus of several hundred before its code evidence counts. A field added only to make a checker look is the restated-evidence burden urzua exists to remove; a record the code cites is a record whose claim can be judged. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A record with cited code and no `Realized-by` field is judged; a record with neither is still not judged; a fixture covers both and the finding text says which evidence was used; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Records with no `Realized-by` judged when cited (`SPEC-2` `0.30`): exercised here by `ADR-2`, `ADR-3`, `ADR-5` and `ADR-6`, whose crates cite them and whose stated `Not started` was corrected to the computed value; a fixture covers cited, uncited and the finding text. **Why:** the Done-when asks for it. Status unchanged; the maintainer closes. | **substantive** |
