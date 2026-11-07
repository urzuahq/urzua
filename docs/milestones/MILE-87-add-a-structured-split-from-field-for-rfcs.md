---
Stable-Id: 01M2178D75JFQTAM32MKF0Z6NB
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: —
---
# 87 — Add a structured Split-from field for RFCs

## What

Add `Split-from` as a declared, comma-separated pointer field on `rfc` (`known_fields`), resolved
by `pointer.resolution` the same as `Implements`/`Derives-from`/`Parent`. Backfill the 3 real
existing instances (`RFC-4`, `RFC-5`, `RFC-6`, all currently `> **Split from RFC-1, ...**` prose
callouts) onto the new field.

## Why

Found live and validated: exactly 4 real occurrences of this exact pattern exist (`RFC-1`'s own
"Split, 2026-07-30" plus 3 RFCs citing it), all dated `2026-07-30`, all consistently worded, the one
target (`RFC-1`) real and `Accepted` — a genuine, recurring, precisely-nameable relationship, not a
one-off. Currently unresolved: a typo'd or renumbered `RFC-1` in any of these three callouts would
go completely undetected, since prose is never parsed. Matches the exact precedent that already
justified `Parent`/`Blocked-on`/`Amends` each getting their own field once a real recurring pattern
appeared, rather than living in prose indefinitely.

## Done means

- `Split-from` is a declared comma-separated pointer field on `rfc` (`known_fields`) resolved by `pointer.resolution` like `Implements`.
- The `> **Split from RFC-1, ...**` callouts on `RFC-1`, `RFC-4`, `RFC-5` and `RFC-6` move to the field.
- A dangling `Split-from` is reported, and `urzua graph` shows the edges.
- The field is exercised in urzua's own `.urzua/config.yaml` and RFCs.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial milestone. **Why:** found live and validated while auditing which currently-prose citations across the corpus are actually recurring, nameable relationships rather than loose context (the same investigation that led to rejecting RFC-21's generic References field). | **structural** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
