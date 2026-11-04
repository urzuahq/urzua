---
Status: Planned
Stable-Id: 01M1Y5JPPE0KSBN3CZKNJ8E9TF
Phase: '3'
Track: accountability-identity
Implements: —
Blocked-on: —
---
# 20 — Non-git identity as Decider / web approval flow

## What

Let a non-technical Decider (a manager, a compliance officer) approve via something other than a git-authored commit.

## Why

Today every Decider is implicitly assumed to be someone who can author a commit; that's not true for every real organization.

## Done means

A Decider with no git identity can record approval through a flow urzua verifies (a signed or otherwise attributable artifact that `check` reads), the approval is attributed in the record without hand-editing, and a fixture shows an unattributed approval refused; the identity model is decided in an ADR first.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |