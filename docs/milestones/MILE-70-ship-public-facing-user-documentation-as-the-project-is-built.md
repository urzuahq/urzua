---
Status: Planned
Stable-Id: 01M1YR7GH547SCN3Z4Q9Z9WCVH
Phase: '1'
Track: user-docs
Blocked-on: —
---
# 70 — Ship public-facing user documentation as the project is built

## What

Public-facing user documentation beyond README/CONTRIBUTING/CHANGELOG: a command reference, the
config schema (record types, header shapes, required fields), the adoption walkthrough, and the
record-type model, written incrementally as each feature ships rather than backfilled once the
backlog is "done." No format/hosting decided yet -- could be a `docs/` directory of plain markdown
GitHub already renders, or a generated site; that choice is separate from committing to write the
content as it ships.

## Why

Documentation written after the fact is a reconstruction, not a record of the reasoning that was
fresh at the time -- the same argument ADR-29 already made for changesets over a hand-written
changelog, extended to user-facing docs generally. This project already has running proof of the
opposite failure: `CHANGELOG.md` currently misdescribes what commands are real because writing it
was deferred past the point anyone had the context fresh (MILE-40). Positioned right after Phase 0
bootstrap specifically so it doesn't repeat that failure at a larger scale once more of the roadmap
ships.

## Done means

- The public documentation set exists beyond the README, contributing guide and changelog: a command reference for every shipped command.
- The public documentation set includes the config schema (record types, header shapes, required fields).
- The public documentation set includes the adoption walkthrough and the record-type model.
- A feature's documentation lands in the change that ships it, and a command with no reference page is reported by a check or listed here.
- The format and hosting are decided in an ADR first.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |