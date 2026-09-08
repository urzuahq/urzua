---
Stable-Id: 01M2178DSHWKHW4BCSW03MPGV3
Status: Planned
Phase: '0'
Track: schema-governance
Blocked-on: —
---
# 88 — Check ADR Amendment sections are dated and preserve the original Decision text

## What

A new `check` rule verifying, for every `Accepted` ADR with one or more `## Amendment (...)`
sections: (1) each is dated in a consistent, parseable format, and (2) the original `## Decision`
section's text is never altered by the same commit that adds an Amendment — the two mechanical
guarantees this project's entire "never silently rewrite an Accepted decision" discipline actually
depends on.

## Why

Found live: `## Amendment (DATE): ...` is used 6 times across 5 `Accepted` ADRs this session alone
(`ADR-14`, `ADR-33`, `ADR-34`, `ADR-36`, `ADR-38`) and is arguably the single most safety-critical
convention in this project, yet **zero rules check anything about it** — no rule verifies dating,
none verifies the original Decision text survives. Validated by audit: every existing instance is
correctly dated and correctly scoped to an `Accepted` record (no violations found), and a spot-check
of `ADR-38`'s actual amendment commit confirmed the Decision text really was left untouched, not
just claimed to be. So the discipline has held so far -- entirely on trust, unlike `revision-log.
change-class-required`, which already gives specs' equivalent convention (a Why-bearing, classified
revision-log entry) real mechanical backing.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial milestone. **Why:** found live and validated while auditing which prose conventions across the corpus are load-bearing enough to deserve mechanical enforcement, following the same investigation that led to rejecting RFC-21. | **structural** |
