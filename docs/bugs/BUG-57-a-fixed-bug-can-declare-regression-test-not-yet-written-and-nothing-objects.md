---
Stable-Id: 01M2W6DTP0TWNCMETZ3YM0G9H5
Status: Open
Found-in: 'Auditing which of this session''s failure modes are checkable rather than only discipline'
Regression-test: 'not yet written -- a `Fixed` bug whose `Regression-test` is a "not yet written" placeholder must be a finding; one naming a real test must not. This record is deliberately its own first case.'
Blocked-on: MILE-98
---
# 57 — A `Fixed` bug can declare `Regression-test: not yet written` and nothing objects

## What was wrong

`AGENTS.md` requires a **planted-violation test, observed failing before the fix**. The `bug` type has
a `Regression-test` field for exactly that claim. Nothing compares the two.

Measured 2026-09-19: **8 bugs marked `Fixed` whose `Regression-test` reads "not yet written"**, across
`BUG-27`, `36`, `37`, `38`, `39`, `53`, `54`, `56`.

They split two ways, and both are findings:

| | |
|---|---|
| **stale field** | `BUG-38`, `39`, `53`, `54` -- the test exists and was observed failing; the field was never updated |
| **absent test** | `BUG-56` -- fixed and verified live, no test written |

## Why nothing caught it

`field.quality` checks whether a required field holds a *real value*, and "not yet written -- a corpus
holding a date-named file must yield 2" is a real, informative sentence. It is simply a claim about
the future stated by a record that says the work is done.

That is the shape this project keeps finding: the engine has both halves and never compares them.
`BUG-49` was a locator naming nothing, `BUG-52` a spec enumerating a set it had drifted from, `BUG-41`
a companion field declared without its pair. This is the same, one field over.

## Fix

A rule pairing `Status` against `Regression-test`: a terminal status with a placeholder regression
test is a finding. The statuses that count as terminal are declared, not inferred -- the
`is_terminal_status` `_ => &[]` lesson.

Worth noting what it cannot do: it detects a *claim* that no test exists, not a *missing* test. A
record naming a test function that was never written would pass. Pairing the field against the source,
as `embodiment.locator-exists` does for locators, is the stronger form and needs the same
caller-supplied existence check.

## References

- AGENTS.md -- the planted-violation requirement this makes checkable.
- BUG-41, BUG-49, BUG-52 -- the same family: two halves the engine holds and never compares.
- MILE-98 -- deferred behind it with the rest, because building this now is one more hardcoded
  comparison to rewrite as a declaration.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** auditing which of this session's failure modes are mechanizable rather than discipline. Skipping the planted-violation test was the most expensive one -- four fixes shipped that compiled, ran and did nothing -- and the corpus already carries a field asserting the test exists, which nothing checks. Eight live instances, four of them stale fields and one a genuinely absent test. | **substantive** |
