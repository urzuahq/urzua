---
Version: '0.5'
Date: 2026-09-08
Status: Accepted
Author: '@beauwilliams'
Subject: 'The `adr` record type -- schema, required fields, and lifecycle for decision records.'
Parent: —
Implements: ADR-10
---
# SPEC-16 — The `adr` record type

## Purpose

`adr` is one of the three founding record types this project is built around, alongside `rfc` and
`spec` — a point-in-time decision fact, never edited in place once `Accepted`, amended via visible,
dated `## Amendment` sections instead (ADR-14). Backfilled the same way `milestone` (SPEC-6), `bug`
(SPEC-9), and `waiver` (SPEC-10) already were, once ADR-41's review found the three founding types
had never gotten this treatment despite being the oldest and most-used types in the schema.

## Schema

```toml
[record_types.adr]
dir = "docs/adr"
required_fields = ["Status", "Date", "Author", "Deciders"]
header_shape = "yaml-frontmatter"
known_fields = ["Embodiment", "Realized-by", "Stable-Id", "Derives-from", "Supersedes / Superseded-by"]
spec = "SPEC-16"
```

| Field | Values | Notes |
|---|---|---|
| `Status` | `Proposed` \| `Accepted` \| `Rejected` \| `Superseded` | Terminal once `Accepted`/`Rejected`/`Superseded` — the decision text itself is then frozen; further evolution is a dated `## Amendment` section, never a silent edit (ADR-14). |
| `Date` | `YYYY-MM-DD` | When decided, not when last touched. |
| `Author` | a real identity | Resolved automatically by `urzua new` (`resolve_identity()`), same tiering `fix --apply` uses (RFC-2 §2). |
| `Deciders` | one or more real identities | Free text today — not yet tool-resolved (MILE-18, rescoped to note `Author` already is). |
| `Embodiment` | `Not started` \| `Specified` \| `Implemented` \| `Verified` \| `Drift detected` \| `Inactive` | Optional; computed-vs-stated consistency checked by `embodiment.consistency` (ADR-18) when `Realized-by` is also present. |
| `Realized-by` | categorized locators (`spec:`/`code:`/`test:`), optional | The evidence `Embodiment` is computed from. |
| `Stable-Id` | a ULID, optional | Backfilled by `migrate ids` (ADR-3/21) for records predating it. |
| `Derives-from` | comma-separated, optional | Points at the RFC(s) this ADR decides. |
| `Supersedes / Superseded-by` | comma-separated or `—`, optional | Reciprocity checked by `relation.supersession-reciprocity`, status-aware (only binds once the claiming record is itself terminal-accepted). |

## Why amendments, not revisions

Distinct from `spec`'s living-document model (ADR-14/SPEC-18): an ADR is a record of what was
decided and why, at the moment it was decided. Silently editing that text would erase the ability to
answer "what did we think originally, and when did reality diverge" — the exact property a
decision-record system exists to guarantee. A visible, dated `## Amendment` section is added instead,
the original Decision text never touched. This session amended `ADR-33`, `ADR-34`, `ADR-36`, and
`ADR-38` this way, each preserving the original Decision alongside dated corrections.

## What's deliberately not built

- **Mechanical `Status` enum validation** — nothing validates `Status` against this closed set today
  for any record type (MILE-46/76 territory), `adr` included.
- **Role rules on `Deciders`** (self-acknowledgement, real identity resolution) — named in SPEC-1's
  original design, not yet built (MILE-18, MILE-55).

## References

- ADR-10 — the decision this spec details: `adr`/`rfc`/`spec` as config-declared profiles of one
  unified core schema, not types the tool's source code hardcodes.
- RFC-1 — the proposal ADR-10 decided; the unified core-plus-profile schema `adr` is an instance of.
- RFC-10 — the closed-header model.
- ADR-3 — stable identifiers, `Stable-Id`'s origin.
- ADR-14 — the amendment-not-edit-in-place model this spec's own "why amendments" section explains.
- ADR-18/32 — Embodiment consistency and drift detection.
- ADR-33/42 — the `yaml-frontmatter` migration this spec's own `header_shape` declaration reflects.
- ADR-41 — the "coherent feature area, decided editorially" principle under which this spec was
  written, backfilled for the founding types last.
- ADR-43 — the `type.no-declared-spec` rule whose live finding for `adr` this spec closes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `adr` — one of the three founding record types — never got a schema spec, unlike `milestone`/`bug`/`waiver`, despite being older and more heavily used than any of them; found live via `type.no-declared-spec`'s own inventory check (ADR-43). | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-08 | Added `Derives-from: RFC-1 (Accepted)`. **Why:** `adr` is a founding type decided by RFC-1, not by any single ADR (unlike `milestone`/`bug`/`waiver`, each pointing at the ADR that decided them) -- this spec cited RFC-1 in prose and References but never backlinked it in the header, the same pointer every other type declares. | **substantive** |
> | 2026-09-08 | Corrected: replaced `Derives-from: RFC-1` with `Implements: ADR-10`. **Why:** the previous entry's premise was wrong -- `ADR-10` is exactly the single decision record `milestone`/`bug`/`waiver`'s own specs each point at (`Implements: ADR-N`), just missed when this spec was first written; `adr` is not an exception to that pattern after all. RFC-1 stays cited in References as the proposal ADR-10 decided. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
