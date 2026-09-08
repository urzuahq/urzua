---
Status: Planned
Stable-Id: 01M1YWKNRK10NQZRKPGW7GT0ZH
Phase: '1'
Track: schema-governance
Implements: ADR-18
Blocked-on: —
---
# 76 — A realization model for foundational decisions with no single-file locator

## What

ADR-18's Embodiment model computes a tier from categorized `Realized-by` locators (`spec:`/`code:`/
`test:`), which works when a decision is realized by specific, nameable files. It has no shape for a
decision realized by the codebase's overall structure rather than any locatable file — ADR-1 ("Rust
as the implementation language") isn't realized by one file, it's realized by the whole Rust
workspace existing at all. `embodiment_consistency` already documents that a record with no
`Realized-by` is skipped as "expected, not a defect" (`rules.rs:526-528`) for a corpus that "hasn't
adopted the field" — but for a genuinely foundational/structural decision, there may never be a
locator to adopt. Decide: does `Realized-by` gain a way to name a diffuse locator (a directory, a
whole-crate reference, or a `structural:` category distinct from `spec`/`code`/`test`), or is
"never mechanically checkable, state the tier by hand and say so" the permanent, correct answer for
this class of decision — the same kind of permanent-not-a-maturity-gap call SPEC-1 already makes for
structural-vs-content-scope?

## Why

Found live: ADR-1, and roughly a dozen other `Accepted` ADRs (2, 3, 5, 6, 9, 11, 12, 13, 14, 15, 25,
33, 34), all state `Embodiment: Not started` with no `Realized-by` field, several of which are
obviously realized today (ADR-1's Rust workspace, ADR-3's stable IDs, ADR-11's waiver parsing code).
`embodiment_consistency` can't flag any of them as stale, because it has nothing to compute against
— not because the values are actually correct. Fixing the stated values by hand (tracked separately)
doesn't close the underlying gap: the next foundational decision will drift the same way, silently,
unless the model itself gains a way to represent this class of realization or explicitly declares it
out of mechanical scope.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
