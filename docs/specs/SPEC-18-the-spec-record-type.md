---
Version: '0.2'
Date: 2026-09-08
Status: Accepted
Author: '@beauwilliams'
Parent: SPEC-1 (v0 CLI).
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
required_fields = ["Status", "Date", "Version"]
header_shape = "yaml-frontmatter"
spec = "SPEC-18"
```

| Field | Values | Notes |
|---|---|---|
| `Status` | `Draft` \| `Accepted` | Unlike `adr`/`rfc`, a spec's `Status` doesn't track `Rejected`/`Superseded` states — a spec that's wrong gets corrected in place (that's the whole point of the living-document model), not superseded the way a frozen ADR would be. |
| `Date` | `YYYY-MM-DD` | The spec's original creation date — stays fixed across revisions (confirmed against `SPEC-1` through `SPEC-15`: none update `Date` on a version bump, only the revision log's own dated rows track when each change landed). |
| `Version` | `0.1`, `0.2`, ... | Bumped on every substantive or structural revision (ADR-14's amendment). |

**`known_fields` is deliberately not declared here yet** — a live, open gap, not an oversight. `SPEC-1`
is the one spec in this corpus that carries `Embodiment`/`Author`/`Derives-from` in its header;
`SPEC-2` through `SPEC-17` don't (MILE-81's own field-set-consistency finding). Declaring
`known_fields` now would mean guessing which side of that split is the actual intended schema —
`Author` becoming required for every spec (matching `adr`/`rfc`'s accountability model), or
`SPEC-1`'s extra fields being its own historical artifact from being hand-authored before this
project's `spec` config existed. MILE-74 (a spec template) is exactly where this canonical field set
gets decided; this spec's `known_fields` stays undeclared until then, and `header.field-set-
consistency` correctly stays silent for `spec` in the meantime (ADR-39's own additive-skip design).

## What's deliberately not built

- **`known_fields`** — see above; blocked on MILE-74.
- **A template** — `urzua new spec` still fails outright (MILE-74); every spec in this corpus,
  including this one, was hand-authored.
- **Mechanical enforcement of "replayable end-to-end"** — ADR-14's amendment requires a spec's body
  to stay a complete, self-sufficient specification of its subject at every revision; nothing
  currently checks this mechanically (and likely can't — it's a content-quality property, the same
  permanent ceiling SPEC-1 already names for structural-vs-content-scope checking generally).

## References

- RFC-1 — the unified core-plus-profile schema `spec` is an instance of.
- RFC-6 — the original living-spec proposal.
- ADR-14 — the append-only-revision-log model (and its amendment: permanent numbers, real Why,
  replayable-end-to-end bodies) this type's entire lifecycle follows.
- ADR-33/42 — the `yaml-frontmatter` migration this spec's `header_shape` reflects.
- ADR-41 — the spec-organization principle this backfill follows.
- ADR-43 — the `type.no-declared-spec` rule whose live finding for `spec` this spec closes.
- MILE-74 — blocks `known_fields` being declared here, and blocks a template existing at all.
- MILE-81 — the field-set-consistency rule that found `SPEC-1`'s own outlier shape live.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `spec` — one of the three founding record types, and the type this very document is an instance of — never got a schema spec; found live via `type.no-declared-spec`'s own inventory check (ADR-43), alongside the same gap for `adr`/`rfc`. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
