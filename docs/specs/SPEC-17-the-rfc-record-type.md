---
Version: '0.4'
Date: 2026-09-08
Status: Accepted
Author: '@beauwilliams'
Parent: SPEC-1 (v0 CLI).
Implements: ADR-10
---
# SPEC-17 — The `rfc` record type

## Purpose

`rfc` is one of the three founding record types, alongside `adr` and `spec` — a proposal, not yet a
decision. Where an ADR records a decision fact, an RFC records the case being made for one, staying
in `Draft`/`Discussion` until an ADR (`Derives-from: RFC-N`) either accepts or rejects it. Backfilled
alongside `SPEC-16` (adr) and `SPEC-18` (spec), for the same reason: the founding types never got the
schema-spec treatment `milestone`/`bug`/`waiver` already had (ADR-41).

## Schema

```toml
[record_types.rfc]
dir = "docs/rfc"
required_fields = ["Status", "Date", "Author"]
header_shape = "yaml-frontmatter"
known_fields = ["Supersedes / Superseded-by", "Amends"]
spec = "SPEC-17"
```

| Field | Values | Notes |
|---|---|---|
| `Status` | `Draft` \| `Discussion` \| `Accepted` \| `Rejected` \| `Superseded` | An RFC's own lifecycle is independent of whether an ADR has decided it yet — `Accepted`/`Rejected` here means the *proposal itself* reached that state, not that a governing ADR necessarily exists (`pointer.resolution` surfaces a Draft-target's status when an ADR derives from it, per RFC-12, but never judges it). |
| `Date` | `YYYY-MM-DD` | When proposed. |
| `Author` | a real identity | Resolved automatically by `urzua new`, same as `adr`. |
| `Supersedes / Superseded-by` | comma-separated or `—`, optional | Same reciprocity model as `adr`. |
| `Amends` | comma-separated, optional | An RFC narrowing or correcting an earlier one without fully superseding it — distinct from `Supersedes`, which replaces outright. |

No `Deciders` field — an RFC is a proposal one person can author; deciding it is what the resulting
ADR's own `Deciders` field records.

## What's deliberately not built

- **A discovery-gate stage before Draft** — explored as an open question (MILE-35), not built.
- **Mechanical `Status` enum validation** — same gap as every other type (MILE-46/76).

## References

- ADR-10 — the decision this spec details: `adr`/`rfc`/`spec` as config-declared profiles of one
  unified core schema, not types the tool's source code hardcodes.
- RFC-1 — the proposal ADR-10 decided; the unified core-plus-profile schema `rfc` is an instance of.
- RFC-10 — the closed-header model.
- ADR-33/42 — the `yaml-frontmatter` migration this spec's `header_shape` reflects.
- ADR-41 — the spec-organization principle this backfill follows.
- ADR-43 — the `type.no-declared-spec` rule whose live finding for `rfc` this spec closes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `rfc` — one of the three founding record types — never got a schema spec; found live via `type.no-declared-spec`'s own inventory check (ADR-43), alongside the same gap for `adr`/`spec`. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-08 | Added `Derives-from: RFC-1 (Accepted)`. **Why:** `rfc` is a founding type decided by RFC-1, not by any single ADR (unlike `milestone`/`bug`/`waiver`, each pointing at the ADR that decided them) -- this spec cited RFC-1 in prose and References but never backlinked it in the header, the same pointer every other type declares. | **substantive** |
> | 2026-09-08 | Corrected: replaced `Derives-from: RFC-1` with `Implements: ADR-10`. **Why:** the previous entry's premise was wrong -- `ADR-10` is exactly the single decision record `milestone`/`bug`/`waiver`'s own specs each point at (`Implements: ADR-N`), just missed when this spec was first written; `rfc` is not an exception to that pattern after all. RFC-1 stays cited in References as the proposal ADR-10 decided. | **substantive** |
