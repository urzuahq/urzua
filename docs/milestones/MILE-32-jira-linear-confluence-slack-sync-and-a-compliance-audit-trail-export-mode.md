---
Status: Planned
Stable-Id: 01M1Y5JX9C69PVW0QPDH8SY5PM
Phase: '4'
Track: integrations
Implements: —
Blocked-on: —
---
# 32 — Jira/Linear/Confluence/Slack sync and a compliance/audit-trail export mode

## What

Two-way sync so a decision can originate from or update an external ticket, and a tuned export for regulated-industry auditors.

## Why

Neither has any RFC coverage today.

## Done means

- A decision can be created from or updated by an external ticket through a two-way sync.
- A conflicting edit is reported.
- A compliance export produces a record an auditor can read with every decision, its status history and its evidence.
- Each integration is behind a declared option and has a fixture.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Removed an appeal to research a reader cannot open; the argument now rests on the absence of any RFC coverage. `Status` unchanged. | **structural** |
