---
Stable-Id: 01M6D2KGWV874476JXN1CXNN8F
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Regression-test: "fix_apply_ids_that_match_no_repair_is_a_failure_naming_the_selector (rust/crates/urzua-cli/tests/check_integration.rs)"
---
# 166 — fix --apply --ids with an entry that matches no repair returns ok and writes nothing

## What was wrong

`urzua fix --apply --ids <x>` selects repairs whose repo-relative path equals `<x>` exactly. An entry
that matched nothing (a typo, or a record id such as `ADR-1`) selected no repair and the run reported
`"status": "ok"`, `repairs: []`, `failed: []`, exit 0, with the file untouched and its repair still
pending. `ok` reads as "checked, nothing to do" -- a command that ran on nothing was indistinguishable
from one that found nothing, which is the no-silent-no-op bug class.

## Why nothing caught it

`fix_apply_on_an_empty_corpus_still_exits_0` pins that an empty population exits 0, and the comment on
`build_fix_report` rightly keeps that. Nothing distinguished "no repairs to apply" from "the selector
named nothing", because no test passed a selector that missed.

## Fix

Each `--ids` entry that selected no pending repair is now a `failed` entry naming it, so the status is
`partial-failure` and the exit code 1, exactly the first-class partial-failure outcome SPEC-8 already
defines for a repair that cannot be written. Nothing is written for it. `--force` ignores `--ids`, so it
reports none. The matching rule is unchanged.

## Notes: a design question left open

The selector form is still a full repo-relative path, as before. SPEC-8 writes `--ids <record,...>`
without defining the form and `--help` says "record". Whether a record id (`ADR-1`, resolved through
the identity index, with a collision or a legacy `NNNN-` filename to decide) should also be accepted is
a SPEC-8 decision, not made here. Until then `--ids ADR-1` fails loudly instead of silently succeeding.

## References

- `SPEC-8` -- apply mode's hard gates and partial-failure outcome.
- `BUG-169` (the `graph`/`explain` disclosure gap) -- same shape: a success status that does not depend
  on the input having reached anything.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed in the same change. **Why:** found by a full-release code review and reproduced with the release binary on a disposable fixture; the regression test was observed failing before the fix and passing after. | **substantive** |
