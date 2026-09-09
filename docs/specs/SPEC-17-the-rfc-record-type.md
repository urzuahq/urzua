---
Version: '0.7'
Date: 2026-09-08
Status: Accepted
Author: '@beauwilliams'
Subject: 'The `rfc` record type -- schema, required fields, and lifecycle for proposals.'
Parent: —
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
known_fields = ["Stable-Id", "Supersedes / Superseded-by", "Amends", "Implements"]
spec = "SPEC-17"
pointer_fields = ["Implements", "Amends"]
narrative_fields = []
```

| Field | Values | Notes |
|---|---|---|
| `Status` | `Draft` \| `Discussion` \| `Accepted` \| `Rejected` \| `Superseded` | An RFC's own lifecycle is independent of whether an ADR has decided it yet — `Accepted`/`Rejected` here means the *proposal itself* reached that state, not that a governing ADR necessarily exists (`pointer.resolution` surfaces a Draft-target's status when an ADR derives from it, per RFC-12, but never judges it). |
| `Date` | `YYYY-MM-DD` | When proposed. |
| `Author` | a real identity | Resolved automatically by `urzua new`, same as `adr`. |
| `Stable-Id` | a ULID, optional | Assigned unconditionally by `urzua new rfc` (ADR-21: every type gets one); backfilled by `migrate ids` for records predating it. |
| `Supersedes / Superseded-by` | comma-separated or `—`, optional | Same reciprocity model as `adr`. |
| `Amends` | comma-separated, optional | An RFC narrowing or correcting an earlier one without fully superseding it — distinct from `Supersedes`, which replaces outright. Declared as one of this type's `pointer_fields` entries (MILE-90/ADR-44) — previously never resolved by anything. |
| `Implements` | comma-separated, optional | Already-generic field (RFC-1) — points at whichever bug or decision this RFC's proposal addresses (e.g. `RFC-23`'s own `Implements: BUG-8`). Added to `known_fields` and declared as this type's other `pointer_fields` entry in the same pass (MILE-90/ADR-44) — previously in real use but never declared, silently tripping `header.field-set-consistency`. |

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
> | 2026-09-08 | Added `Stable-Id` to `known_fields`. **Why:** found live creating the first `rfc` records via `urzua new rfc` (RFC-18/19/20) -- `render_synthetic_yaml` assigns every type a `Stable-Id` unconditionally (ADR-21), but no prior `rfc` had ever been created through the tool, so this gap sat unexercised until now. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | Added `Implements` to `known_fields`, declared `pointer_fields = ["Implements", "Amends"]`/`narrative_fields = []` in config (MILE-90/ADR-44). **Why:** `RFC-23`'s own header (`Implements: BUG-8`) had been silently tripping `header.field-set-consistency` since it was written -- `rfc` never declared `Implements` as known despite already using it in practice. `Amends` had also never been resolved by anything; declaring it a `pointer_fields` entry makes it real. | **substantive** |
