---
Stable-Id: 01M2W6DTP0TWNCMETZ3YM0G9H5
Status: Fixed
Found-in: 'Auditing which of this session''s failure modes are checkable rather than only discipline'
Regression-test: 'rust/crates/urzua-core/src/rules/status.rs::a_declared_pending_value_does_not_satisfy_a_requirement_observed_failing, rust/crates/urzua-core/src/rules/field.rs::pending_is_legitimate_until_the_required_status_observed_failing'
Blocked-on: —
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
> | 2026-11-04 | `Status` set to `Fixed`; `Blocked-on` cleared; `Regression-test` names the covering tests. Fixed in #173 (`MILE-132`: `status.required-field-missing` reads the type's declared `statuses.require` and `statuses.pending`, so a `Fixed` bug whose `Regression-test` is a placeholder is an error) and #224 (`MILE-169`: the seven `Fixed` bugs that held a placeholder were given their real test, and `field.pending` stays quiet only until the status that requires the field). Verified on main: setting `BUG-180`'s `Regression-test` to `not yet written -- later` makes `urzua check` report `status.required-field-missing` (error) and `field.pending`. **Why:** the pairing this record asked for is built as a declaration, not a hardcoded comparison, so nothing waits on `MILE-98`. | **substantive** |
