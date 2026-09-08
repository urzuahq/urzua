---
Version: '0.3'
Date: 2026-09-07
Status: Accepted
Author: '@beauwilliams'
Implements: ADR-11
Parent: SPEC-1 (v0 CLI).
---
# SPEC-10 — The `waiver` record type

## Purpose

A reviewed exception to a rule, as a first-class record — never a config-level ignore list. An
ignore list is unreviewable and grows forever; a waiver is a normal record with a reason and an
optional expiry, reviewable in the same diff as everything else. A waived finding stays listed in
`check`'s output — only excluded from the blocking exit code — so a waiver never makes a problem
invisible, only non-blocking.

## Schema

```toml
[record_types.waiver]
dir = "docs/waiver"
required_fields = ["Rule", "Scope", "Reason"]
header_shape = "yaml-frontmatter"
known_fields = ["Stable-Id", "Expires"]
spec = "SPEC-10"
```

| Field | Values | Notes |
|---|---|---|
| `Rule` | a rule id, e.g. `field.quality` | Which rule this waiver covers. |
| `Scope` | a file path, or `*` | `*` covers every file the rule examines for this waiver's `Rule`. |
| `Reason` | free text, required | Why this exception is reviewed and accepted, not left to drift — the whole reviewability point of not using an ignore list. |
| `Expires` | `YYYY-MM-DD`, optional | No expiry means the exception is asserted **structural** (the same shape as SPEC-1's permanent content-scope ceiling). A stated expiry means the waiver **reverts to blocking automatically** once passed — no separate "expired" state anyone has to notice or configure. |

No `Status` field: a waiver's lifecycle is entirely computed from `Expires` (`is_active(today)`),
not separately authored — unlike every other record type, there is nothing for a human to declare
that isn't already implied by the date.

## Matching semantics

A waiver **covers** a finding when `Rule` matches the finding's rule id exactly, and either `Scope`
is `*` or matches the finding's file path exactly (`urzua-core::waiver::Waiver::covers`). A waiver
with a passed `Expires` is treated as though it doesn't exist (`is_active`) — comparison is a plain
ISO-8601 string comparison against wall-clock "today," which RFC-15 §1 notes as the first place this
schema reads real time rather than only corpus content.

`check` calls `load_waivers` (filters discovered records to `record_type == "waiver"`, skipping any
missing `Rule` or `Scope` rather than crashing the run — an unparseable waiver waives nothing, which
fails toward more findings, not fewer) then `apply_waivers`, which annotates matching findings with
`waived: <waiver's own path>` and never removes them from the report. `status`/`blocking` are
computed from non-waived findings only.

## What's deliberately not built

- **Role enforcement on granting a waiver** (does it need the same Decider rigor as an ADR) — named
  as an open question in ADR-11, deferred with RFC-1 §1e's reciprocity work.
- **A `check` rule verifying a waiver's own `Rule` value actually names a real rule id** — a typo'd
  `Rule` value currently waives nothing silently (fails toward more findings, per the design above),
  but nothing flags the typo itself as likely a mistake.

## References

- ADR-11 — the decision this spec details: a waiver as a first-class record, not an ignore list.
- RFC-15 — the original proposal, including the waiver-as-record shape and `Expires`'s wall-clock
  read as this schema's first.
- MILE-59 — shipped the parsing/matching logic (`waiver.rs`) but never actually registered the type
  in `.urzua/config.toml`, so `urzua new waiver` failed outright until that gap was found and fixed
  the same day this spec was written.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. **Why:** `waiver` (ADR-11) was documented only as an ADR while `milestone`, the same shape of decision (a configured record type), got a matching spec (SPEC-6) — MILE-77 named this inconsistency and this spec resolves it for `waiver`. | **structural** |
> | 2026-09-08 | Header shape declared as `yaml-frontmatter` (ADR-33/42 amendment) -- `waiver` never declared `header_layout` (no records exist yet to have settled on a sub-format), so nothing to remove here, unlike the other five types. `spec = "SPEC-10"` declared in config, closing `type.no-declared-spec`'s live finding for this type (ADR-43). | **substantive** |
> | 2026-09-08 | Bumped to `0.3`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
