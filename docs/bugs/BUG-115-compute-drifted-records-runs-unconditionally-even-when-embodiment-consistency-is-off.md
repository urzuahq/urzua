---
Stable-Id: 01M34QAQ218SAHK6Z8Z2DXYQPR
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "not applicable -- an efficiency fix with no observable behavior change; verified by a real-corpus check run reporting identical findings"
---
# 115 — compute_drifted_records runs unconditionally even when embodiment.consistency is off

## What was wrong

`check.rs` called `compute_drifted_records` unconditionally, before any rule-gating check.
`embodiment.consistency` is its only consumer, and it can be configured `Off` like any other rule
(`ADR-53`). For every record carrying a `Realized-by` field, this function spawns several `git`
subprocesses (`commit_for_line`, `last_commit_for_path`, `commit_strictly_before`) — work discarded
entirely when the rule that would read it isn't enabled. `read_claim_files`'s call, immediately above
it, already self-gates on `claim.status-agreement`'s level; `compute_drifted_records` didn't follow
the same pattern.

## Why nothing caught it

Nothing tests that a declined rule's supporting I/O is skipped, only that the rule's own execution is
gated (`gate::gated`) — the gate covers rule dispatch, not the caller-side data it's handed.

## What changed

`compute_drifted_records` is now called only when `embodiment.consistency`'s configured level isn't
`Off`; otherwise `drifted` is an empty set, matching what an `Off` rule would compute anyway (it never
reads it).

## References

- The `claim_paths`/`read_claim_files` gate immediately above this call site — the pattern this fix
  now matches.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review; verified as a no-op via a real-corpus `check` run reporting identical findings before and after. | **substantive** |
