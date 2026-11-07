---
Status: Planned
Stable-Id: 01M1Y5JKGMK9VX40WZQFBQ6TTR
Phase: '1'
Track: governance-process
Implements: RFC-12
Blocked-on: —
---
# 14 — Decide RFC-12: decision-before-implementation gate

## What

Refuse the merge, not the edit.

## Why

Still Draft; genuinely undecided.

## Done means

- An ADR decides whether merges are refused when a change lands ahead of its decision (the merge, not the edit), names the CI step that enforces it, and `RFC-12` points at it.
- If adopted, a planted change citing an unaccepted decision fails that step in urzua's own `.github/workflows/`, and a decided one passes. — evidence: code:.github/workflows/checks.yml, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#this_repositorys_own_gate_holds_a_source_change_until_a_decision_fix_or_link_changes@sha256:5fa13c8a322e9de1b23af5431b3e78dc72f40037379eb0f7d5698957d21ef9a2

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
