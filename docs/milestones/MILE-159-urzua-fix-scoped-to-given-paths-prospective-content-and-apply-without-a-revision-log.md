---
Stable-Id: 01M6JKNWP4NMYSTVM6QQNMV9P9
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 159 — Urzua fix scoped to given paths, prospective content, and apply without a revision log

## What

Make `urzua fix` accept a path argument or prospective content, make `--apply` work for records that have no revision log section, and give the repair list a stable machine-readable form for an editor hook.

## Why

`urzua fix` is whole-corpus (several seconds on a few hundred records) with no path argument, and `--apply` requires a revision log section the adopter's records do not have, so there is no equivalent of a quick single-file refresh. An editor hook must parse the whole repair list to find its own file. Authoring speed is the point of `MILE-103`. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

`urzua fix <path>` examines and repairs only that path; prospective content on stdin is judged without writing; `--apply` does not need a revision log section; a single-file run completes in well under a second on a large corpus; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
