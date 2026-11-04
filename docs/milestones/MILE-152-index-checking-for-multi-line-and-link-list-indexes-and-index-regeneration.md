---
Stable-Id: 01M6JKNRESRV32NBGDRTQD3M1A
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 152 — Index checking for multi-line and link-list indexes, and index regeneration

## What

Extend index checking to accept entries spread over several lines (a YAML index), a `{title}` that excludes the id, and an `{id}` that need not be a visible token; and add a way to regenerate an index from the records.

## Why

Index checking today reads one line per entry, treats `{title}` as the whole H1 including the id, and requires `{id}`, so a list of linked entries needs a hidden comment to satisfy it. Nothing regenerates an index, which the adopter's old tool did on every audit; hundreds of index lines would otherwise be maintained by hand or by a side script, the duplication urzua is meant to remove. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A multi-line index entry shape is declared and checked; `{title}` can mean the H1 without its id; an index entry needs no hidden id token; a command regenerates a declared index from the records and a second run is a no-op; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: `index.freshness` reads multi-line entry formats (lines separated by `\n`) and formats with `{file}` and no `{id}`; `{title}` excludes the record's own id as well as a leading number; and `urzua index [--apply]` regenerates the declared indexes deterministically (a second run reports every artifact `current`). Exercised in this repository's own `.urzua/config.yaml`: `docs/adr/README.md` (an id-less link list) and `docs/specs/README.md` (a two-line entry) are generated and checked. Documented in `SPEC-2` and `SPEC-3`. The command is a new subcommand rather than a `urzua fix` action because fix repairs a record's own fields. `Status` is untouched. **Why:** the Done-when asks for a declared multi-line shape, an id-less entry, a regeneration command that is a no-op on a second run, and own-repo exercise. | **substantive** |
