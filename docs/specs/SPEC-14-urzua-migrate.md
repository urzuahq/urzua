---
Version: '0.3'
Date: 2026-09-07
Status: Accepted
Author: '@beauwilliams'
Subject: '`urzua migrate ids`/`urzua migrate schema` -- backfilling stable IDs and previewing schema changes.'
Implements: ADR-21, ADR-22
Parent: SPEC-1
---
# SPEC-14 — `urzua migrate`

## Purpose

Migrates the corpus itself — two genuinely different operations under one verb, not one command
with mode flags: backfilling stable identifiers onto existing records, and previewing what a
newly-required field would break before it's ever added to config.

## `urzua migrate ids`

Backfills a `Stable-Id` header field (ULID, ADR-21) into every record lacking one, retaining the
filename's current number as the display number unchanged — cross-references keep resolving by
filename-derived number exactly as they do today; nothing about `pointer.resolution` changes.
Dry-run by default; `--apply` writes.

**Encoding**: ULID — 128-bit, time-ordered, lexicographically sortable, collision-free without
coordination (the property concurrent agent branches need). Backfilled IDs are stamped with the
migration run's own time, not each record's original git history — ADR-3 §6's proposal to derive
backfill timestamps from git history is a named, tracked simplification, not silently dropped,
since nothing in the schema currently reads or sorts by a stable ID's embedded timestamp.

`urzua-id::StableId::generate()` wraps `ulid::Ulid::generate()`.

## `urzua migrate schema --report --field <Name>`

The only implemented tier of a three-tier design (report, waiver-assist, apply). Lists every record
lacking a real value for `Name` — using the same blank/placeholder/pending/present classification
`field.quality` already applies to declared required fields, against a field that isn't declared
yet — without touching config or any record. Read-only, same as `check`. Errors with exactly what's
missing if invoked without both `--report` and `--field <Name>`, rather than doing nothing silently.

## What's deliberately not built

- **`migrate schema --assist-waivers`** — generating a draft waiver (placeholder `Reason`, human
  edits before commit) for every record a newly-required field would break. Blocked on deciding
  whether a generated draft preserves the review property RFC-11's waiver model depends on, or
  whether only a human-authored waiver (SPEC-10) does.
- **`migrate schema --apply`** — delegating to `urzua fix` (SPEC-8) for whichever fields happen to
  be tool-writable. Structurally, almost none are: most newly-required fields (e.g. `Reviewers`)
  need human authorship, the same eligibility question SPEC-8 already answers for `Embodiment`.
  `fix`'s own apply mode existing (SPEC-8) makes this delegation technically possible now, but the
  eligibility question — which fields, if any, would ever qualify — is still open.

## References

- ADR-21 — the ULID encoding decision and its named git-history-timestamp simplification.
- ADR-22 — report-mode-only, and why the other two tiers aren't built yet.
- ADR-3 — the stable-identity/display-number separation `migrate ids` backfills onto existing
  records.
- SPEC-8 — `fix`'s eligibility test, which `migrate schema --apply` would need to delegate to.
- SPEC-10 — the waiver record shape `--assist-waivers` would need to respect.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, bundling `migrate ids` and `migrate schema` since SPEC-1 already treats them as one feature area under one verb. **Why:** MILE-77 found `migrate` documented only as two separate ADRs while comparable-complexity command areas (`check`, `init`) had specs. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
