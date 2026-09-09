---
Stable-Id: 01M23VWVTFGHCT37FXR977BRQV
Status: Done
Phase: '0'
Track: schema-governance
Implements: BUG-10
Blocked-on: —
---
# 91 — Correct spec Parent values, add required Subject field, backfill all specs

## What

Two changes to `spec`'s own schema, landed together since both touch every spec's header:

- `spec` gains a new required field, `Subject`: one line, free text, no cross-reference or backlink
  semantics -- what feature, record type, schema, or practice the spec is actually about, readable
  at a glance without opening the full `Purpose` section. Added to `.urzua/config.toml`'s
  `required_fields` for `spec`, backfilled onto all 18 real spec files in the same commit (a required
  field and its corpus backfill never land separately, same discipline as `MILE-74`'s `Author`
  backfill).
- `Parent` corrected per `BUG-10`'s audit: cleared to `—` on the seven specs where it never expressed
  a real relationship (`SPEC-6`/`9`/`10`/`16`/`17`/`18`/`19`), kept on the ten where `SPEC-1`'s own
  prose documents a genuine split-off relationship.
- `SPEC-18` updated to document `Subject` as required and to state `Parent`'s real, narrower meaning
  now that its data is corrected.
- `.urzua/templates/spec.md` corrected at the source: its throwaway header hardcoded
  `Parent: SPEC-1 (v0 CLI).` as literal boilerplate -- discarded by the live code path
  (`render_synthetic_yaml` only fills `required_fields`) but still the thing a human or agent would
  copy by eye when hand-authoring a new spec, which is almost certainly how the wrong default
  actually propagated. Replaced with an instructional comment.

## Why

`BUG-10` found `Parent: SPEC-1` was a mechanically-applied default on 7 of 18 specs, not a real
relationship -- their actual lineage was already fully expressed by their own `Implements`/
`Derives-from` fields. Fixing wrong data and adding a genuinely useful new field both touch every
spec's header, so they land in one pass rather than two separate corpus-wide edits.

No preceding RFC: this is the same kind of direct, `spec`-type-own schema decision `MILE-74`
made through live discussion without one -- a config/schema-only change to an already-config-driven
type, no new mechanism or rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-09 | Initial milestone, `Status: Done`. **Why:** `BUG-10`'s audit plus a live decision to add `Subject` as a required field -- both executed together since they touch the same 18 files' headers. | **substantive** |
