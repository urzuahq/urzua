---
Stable-Id: 01M33QPP9RM9CSPXTATYEXPX42
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "not applicable -- a comment-wording fix has no behavior to regress"
---
# 107 — comments narrate bug-ticket history and use temporal language against the comment convention

## What was wrong

Several comments added across this release used temporal language ("is now", "the count is now
unit-bearing", "this release") to describe a change relative to its prior state, rather than stating
the current invariant plainly — against the project's own comment convention (never restate a change
relative to time; state the fact). Instances: `check_integration.rs` (four call sites describing what
a population field "now" says), `rules.rs:478` ("this release").

## Why nothing caught it

The convention is enforced by review, not by a rule the engine runs on itself, so a comment written
while a change was fresh in mind naturally described it as a change rather than as a fact.

## What changed

Each flagged comment was reworded to state the invariant directly, without a "before/after" frame.
`property.rs`'s own flagged instance had already been corrected during this same release's earlier
work, verified by direct inspection before making any further edit there.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed and fixed in one pass. **Why:** found by a full-release code review; wording-only, no behavior to verify beyond the existing test suite staying green. | **structural** |
