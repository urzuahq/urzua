---
Stable-Id: 01M6JW42KJAWDCS570PJFFZM61
Status: Open
Found-in: 'a re-run of urzua over a real adopter corpus (2026-11-04)'
Regression-test: 'planned: a test that fails before the fix and passes after, plus the same case exercised through urzua check in this repository'
---
# 187 — Records exempted by exempt_statuses still trigger config.scope-matches-nothing

## What was wrong

A rule's `exempt_statuses` setting sets a record aside by design, yet `config.scope-matches-nothing` counts the exempted records as candidates the configuration does not reach and warns that the rule was handed candidates it cannot examine. The warning is false: the exemption is a declared input, not a misconfiguration. Under `fail_on: warning` it fails the build and the only remedy is a standing waiver. This is the same class as `BUG-186`.

## Why nothing caught it

The `exempt_statuses` tests asserted that an exempt record is skipped; none ran `config.scope-matches-nothing` over the rule's execution, so the outcome chosen for an exempted record (unreachable rather than eligible and unexamined) was never compared with how the meta-rule reads it. `BUG-186` and `MILE-158` fixed the same trap for external ids and diff exclusions, each reviewed one rule at a time.

## References

Same classification as `BUG-186`: an exempted record is eligible and unexamined (set aside on purpose), not out of scope. Done when the change is exercised by an `exempt_statuses` declaration in urzua's own `.urzua/config.yaml` and corpus with no waiver, and `urzua check` over this repository reports it without the warning.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this defect. | **substantive** |
