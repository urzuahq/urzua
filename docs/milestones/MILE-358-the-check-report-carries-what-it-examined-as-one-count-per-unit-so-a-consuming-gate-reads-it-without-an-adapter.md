---
Stable-Id: 01M82KGY405G1ZRC22FNJEJ1Q6
Status: Planned
Phase: '1'
Track: integrations
Blocked-on:
---
# 358 — The check report carries what it examined as one count per unit, so a consuming gate reads it without an adapter

## What

`urzua check` reports what it examined as an object with one count per unit, `examined: {files: N}`,
using the unit names a consuming gate shares (`files`, `tests`, `packages`), so a tool that gates
on urzua's report reads it directly instead of through an adapter that renames `files_examined`.
Whether `files_examined` stays as an alias, and for how long, is decided as part of this work.

## Why

A gate tool that runs `urzua check` as a check without an adapter (a "native" tool) settled on
`examined` as an object of per-unit counts in its own design review on 2026-11-22. urzua's report
has a top-level `files_examined` integer (`RFC-3`, `SPEC-2`, `SPEC-26`), so today the gate needs a
mapping for the one field that proves a check did not run on nothing. Renaming a report key is a
change to the output contract every consumer reads, so the shape and any alias are a recorded
decision, not a silent edit. A `describe` answer (name, scopes accepted, metrics reported) is the
other half of "native"; its shape is not settled yet and is not part of this milestone.

## Done means

- The decision on the key's shape and on keeping `files_examined` as an alias is recorded in an ADR, or in an amendment to the record that governs the report's shape, citing this milestone.
- `urzua check` reports `examined` with a `files` count equal to what `files_examined` reports today, shown by a test over a corpus.
- `SPEC-2` and every other spec or reference page that names `files_examined` describe the decided shape, each with a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
