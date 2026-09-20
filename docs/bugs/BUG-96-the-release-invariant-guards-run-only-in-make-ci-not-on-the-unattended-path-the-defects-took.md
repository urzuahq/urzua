---
Stable-Id: 01M30JTRERM7EJHW8BWNP3NC4S
Status: Fixed
Found-in: "Round 11 of the 0.4.0 review, against nine PRs never reviewed in aggregate"
Regression-test: ".github/workflows/ci.yml, job `release invariants` -- and .github/scripts/release-invariants.test.sh"
---
# 96 — The release invariant guards run only in make ci, not on the unattended path the defects took

## What was wrong

`BUG-93` added two guards over facts nothing checked, and wired them into `make ci` only.
`grep -rn 'release-invariants' .github/workflows/` returned nothing.

So they fired when a human ran `make ci` locally -- and the script's own header states why it exists:
*"A title changing on an already-open PR is not something anyone re-reads, so it went unnoticed
twice."* That is the unattended case. `BUG-92` reached `main` through a branch cut from `release`;
no local `make ci` was in that path either.

**A guard that fires only where the failure did not happen is a check reporting success without
looking** (`ADR-55`), which is the class `BUG-93` was written to close.

Now a required CI job, with full history so the tag comparison has something to compare against.

## Also fixed

The version guard treated an undeterminable tag as agreement: with `latest_tag` empty it reported
`invariants hold`. A shallow checkout or a repository without tags would have passed silently.
*"I could not determine the newest tag"* is not *"the versions agree"* -- the distinction `doctor`'s
`ci-wired` check got right in this same release (`BUG-91`). It is a failure now, with its own planted
case.

## Why nothing caught it

`BUG-93` was written and verified by running `make ci`, which is the one context where the guard
does fire. The question "does this run where the defect happened?" was never asked -- and it is the
same question the guard itself asks about `urzua check`.

## References

- `BUG-93`, the guards; `BUG-92`, the defect that reached `main` unattended.
- `BUG-91`, whose `ci-wired` check draws the could-not-tell distinction correctly.
- `ADR-49`, the release guard this sits beside; `ADR-55`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
