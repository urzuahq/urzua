---
Version: '0.4'
Date: 2026-09-08
Status: Accepted
Author: '@beauwilliams'
Parent: SPEC-1
Implements: ADR-10
---
# SPEC-18 — The `spec` record type

## Purpose

`spec` is one of the three founding record types — the type this very document is an instance of.
Distinct from `adr` (a frozen, point-in-time decision fact) and `rfc` (a proposal not yet decided):
a spec is a **living, current-truth document**, edited in place under ADR-14's model — a substantive
or structural change is a `Version` bump plus a Why-bearing revision-log entry on the same spec, its
own number permanent per subject (ADR-14's amendment). Backfilled alongside `SPEC-16`/`SPEC-17`,
closing the last of the three founding types' schema-spec gap (ADR-41, ADR-43).

## Schema

```toml
[record_types.spec]
dir = "docs/specs"
required_fields = ["Status", "Date", "Version", "Author"]
header_shape = "yaml-frontmatter"
known_fields = ["Stable-Id", "Embodiment", "Derives-from", "Implements", "Parent"]
spec = "SPEC-18"
```

| Field | Values | Notes |
|---|---|---|
| `Status` | `Draft` \| `Accepted` | Unlike `adr`/`rfc`, a spec's `Status` doesn't track `Rejected`/`Superseded` states — a spec that's wrong gets corrected in place (that's the whole point of the living-document model), not superseded the way a frozen ADR would be. |
| `Date` | `YYYY-MM-DD` | The spec's original creation date — stays fixed across revisions (confirmed against `SPEC-1` through `SPEC-15`: none update `Date` on a version bump, only the revision log's own dated rows track when each change landed). |
| `Version` | `0.1`, `0.2`, ... | Bumped on every substantive or structural revision (ADR-14's amendment). |
| `Author` | a real identity | Resolved automatically by `urzua new`, same as `adr`/`rfc` (MILE-74's accountability decision, MILE-78's precedent). Required as of this version — backfilled onto every pre-existing spec that lacked it. |
| `Stable-Id` | a ULID, optional | Assigned unconditionally by `urzua new spec` (ADR-21: every type gets one), first exercised by `SPEC-19` — no earlier spec was ever created through the tool. |
| `Embodiment` | optional | `SPEC-1`'s own field, kept as a project-wide optional convention rather than treated as its historical baggage (MILE-74) — any spec may use it, none but `SPEC-1` currently do. |
| `Derives-from` / `Implements` / `Parent` | comma-separated, optional | The three pointer fields already in real use across this corpus (this very spec's own header carries `Implements` and `Parent`); resolved by `pointer.resolution` regardless of per-type declaration. |

MILE-74 decided this field set: `Author` required (matching `adr`/`rfc`'s accountability model, not
`SPEC-1`'s historical-artifact reading), `Embodiment`/`Derives-from` a project-wide convention rather
than `SPEC-1`-specific, `Implements`/`Parent` declared since every spec already carries them.

## What's deliberately not built

- **Mechanical enforcement of "replayable end-to-end"** — ADR-14's amendment requires a spec's body
  to stay a complete, self-sufficient specification of its subject at every revision; nothing
  currently checks this mechanically (and likely can't — it's a content-quality property, the same
  permanent ceiling SPEC-1 already names for structural-vs-content-scope checking generally).

## References

- ADR-10 — the decision this spec details: `adr`/`rfc`/`spec` as config-declared profiles of one
  unified core schema, not types the tool's source code hardcodes.
- RFC-1 — the proposal ADR-10 decided; the unified core-plus-profile schema `spec` is an instance of.
- RFC-6 — the original living-spec proposal.
- ADR-14 — the append-only-revision-log model (and its amendment: permanent numbers, real Why,
  replayable-end-to-end bodies) this type's entire lifecycle follows.
- ADR-33/42 — the `yaml-frontmatter` migration this spec's `header_shape` reflects.
- ADR-41 — the spec-organization principle this backfill follows.
- ADR-43 — the `type.no-declared-spec` rule whose live finding for `spec` this spec closes.
- MILE-74 — decided this spec's field set (`Author` required, `Embodiment`/`Derives-from` project-
  wide, `Implements`/`Parent` declared) and built `.urzua/templates/spec.md`.
- MILE-81 — the field-set-consistency rule that found `SPEC-1`'s own outlier shape live.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `spec` — one of the three founding record types, and the type this very document is an instance of — never got a schema spec; found live via `type.no-declared-spec`'s own inventory check (ADR-43), alongside the same gap for `adr`/`rfc`. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-08 | Corrected: the previous bump changed `config.toml` and this spec's own header but left the Schema section, field table, and "what's deliberately not built" describing the old, undecided state -- rewrote them to match the actual decided schema (`known_fields` declared, `known_fields`-is-undeclared section removed). Added `Implements: ADR-10` -- the actual decision record for `adr`/`rfc`/`spec` as configured profiles, missed in favor of a `Derives-from: RFC-1` pointer that skipped past it. | **substantive** |
> | 2026-09-08 | Added `Stable-Id` to `known_fields`. **Why:** found live creating the first-ever tool-generated spec (`SPEC-19`) -- `render_synthetic_yaml` assigns every type a `Stable-Id` unconditionally (ADR-21), but no `spec` had ever been created through `urzua new` before MILE-74 made that possible, so this gap sat unexercised until now. | **substantive** |
