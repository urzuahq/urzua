---
Status: Planned
Stable-Id: 01M2VN4C5S5YSDEHK5P136XBAQ
Phase: '0'
Track: governance-process
Implements: SPEC-1
Blocked-on: —
---
# 99 — The pre-push hook runs `urzua check`, not only fmt and clippy

## What

`make hooks-install` installs a pre-push hook that runs `cargo fmt` and `cargo clippy`. It does not
run `urzua check`, so the tool that governs this corpus is enforced in CI and nowhere else.

## Why it is Phase 0

`SPEC-1`'s first success criterion is explicit: *"It runs green in at least two real target codebases
... **via pre-push hook and CI**"*. Half of that is built. The hook is not a convenience -- it is named
in the bar Phase 0 exits on.

It also closes a real loop. A record written on a branch is checked only after it reaches a pull
request, so the feedback arrives once the work is packaged rather than while it is being written. Two
records were pushed this week with headers `check` rejected, and both round-trips would have been
avoided locally.

## The decision this needs

**What the hook does with a non-blocking finding.** The corpus currently reports 61 findings at
`blocking: false`; a hook that refuses a push on any finding would refuse every push today. Options,
not yet chosen:

- Block only on `blocking: true`, matching what CI does -- simplest, and the hook adds nothing CI does
  not already catch.
- Block on any *new* finding relative to the merge base -- catches regressions without demanding the
  backlog be cleared first, and needs a baseline `MILE-49` is already about.

The second is more useful and more work. This milestone owns choosing, not both.

## Not the same as making the corpus clean

61 findings at `warn` is a separate question. This milestone is about *where* the check runs, not about
what level the rules are declared at.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s first success criterion names the pre-push hook alongside CI, and only CI was built. Surfaced by asking what Phase 0 actually exits on rather than reading the milestone list, which did not contain this. | **substantive** |
