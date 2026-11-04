---
Stable-Id: 01M6HZ0Q0A3NTHSZ5WV1A9GJHA
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
---
# 143 — Check that cited paths and issue or PR numbers exist

## What

A warning-severity check over the citations in a record's references: a backtick-quoted repository path must exist inside the repository, and an issue or pull request reference (`#N` or a `/pull/N` link) must resolve to a real issue or pull request. It needs network access and a token, so it is opt-in and reports `not-run` when either is missing instead of passing.

## Why

The adopter checks both in the References section of every decision, proposal and spec record, as warnings. `embodiment.locator-exists` covers only `Realized-by` locators, and `MILE-37` and `MILE-108` are adjacent (README links, source comments) rather than the same check. `check` is otherwise offline and deterministic, so the network dependency has to be explicit and separable from the blocking run.

## Done means

A cited path that does not exist and a cited issue number that does not exist each produce a warning; a run without a token reports `not-run` with the reason; the rule never makes the default offline `check` non-deterministic; batching and rate-limit handling are covered by a test against a recorded fixture.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter warns on cited paths and issue or PR numbers that do not exist, and urzua checks neither. | **substantive** |
