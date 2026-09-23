---
Stable-Id: 01M35XZVT5BX5XWPGCFQ6X4AA4
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "a_scoped_invocation_still_reports_a_claim_finding_observed_failing, check_integration.rs"
---
# 121 — check's path-scope filter still drops a claim.status-agreement finding whose claim file falls outside the requested scope

## What was wrong

`check.rs`'s post-run scope filter (`BUG-67`) keeps a finding only if its file is the config file or
falls under one of the requested path scopes. That's correct for every rule reporting on a record, but
`claim.status-agreement`'s findings name a claim file under `claim_paths` (e.g. `changes/`), which no
record-type `dir` scope covers. A scoped invocation — `urzua check docs/adr`, not unscoped `check` —
silently dropped a real false-claim finding, exactly as `BUG-86` already documented for this
repository's own `make records` invocation.

`BUG-86`'s own fix changed the *caller* (this repository's Makefile) to run `check` unscoped instead of
fixing the filter, and said so explicitly: *"The same blindness applies to any rule whose findings name
a file outside `docs/`."* That general defect was never closed — any other repository, or any narrower
invocation of `check` here, still hit it.

## Why nothing caught it

`BUG-86` scoped its own fix to this repository's gate; nothing generalized the exemption the way
`config_path` already gets one line above the same filter.

## What changed

Added `rules::RULES_REPORTING_OUTSIDE_THE_CORPUS`, listing `claim.status-agreement`, and exempted any
finding from a listed rule the same way a config-file finding already is. Follows the same
declared-list pattern as `IDENTITY_DEPENDENT_RULES`.

## References

- `BUG-67` -- the scope filter this bug is a gap in, correct for every other rule.
- `BUG-86` -- named the same general defect, fixed only this repository's own invocation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review; a planted-violation test (a scoped `check docs/adr` over a false claim under `changes/`) observed failing before the fix and passing after. | **substantive** |
