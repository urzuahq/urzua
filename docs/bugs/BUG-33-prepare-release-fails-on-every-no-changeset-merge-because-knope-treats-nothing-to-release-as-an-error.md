---
Stable-Id: 01M2MMFX94TCYK8M5H9KQ2VEMW
Status: Open
Found-in: 'checking why the prepare-release run that followed PR #37 was red -- knope exited 1 with `releases::no_release`, and #37 was a `no-changeset` PR with nothing to release'
Regression-test: 'not yet written -- needs a workflow-level assertion that a no-changeset merge to main leaves prepare-release green, which cannot be unit-tested and wants either `allow_empty` or a guard step whose skip is itself visible'
---
# 33 — prepare-release fails on every no-changeset merge, because knope treats nothing-to-release as an error

## What was wrong

`prepare-release.yml` runs on every push to `main`. When the merged change added no
`.changeset/*.md` fragment, `knope prepare-release` exits 1:

```
Running step PrepareRelease(PrepareRelease { allow_empty: false, .. })
Using commits since tag v0.2.0
Error: releases::no_release
  × No packages are ready to release
```

Observed on the merge of PR #37, which carried the `no-changeset` label. The label is the repo's
own sanctioned way of saying "this change has nothing to tell an installer" — and every PR that uses
it honestly now turns the release workflow red.

`SPEC-20` asserts the opposite:

> **No `continue-on-error`**: the workflow's `if:` guard already skips the one expected no-op — its
> own release commit — so anything reaching the knope step is a real failure.

That is false. The guard skips the release commit. It does not skip a `no-changeset` merge, and
`no-changeset` is routine — CI-only changes, docs-only changes, and every record-keeping PR.

## Why nothing caught it

`BUG-27` removed `continue-on-error: true` from this step, correctly: it had been hiding a six-day
release outage. But the removal assumed the only legitimate no-op was the release commit, and that
assumption was written into `SPEC-20` rather than tested. Between `BUG-27` landing and this being
noticed, no `no-changeset` PR merged to `main`, so the case never arose.

The failure direction is right — a false alarm is safer than the silent success `BUG-27` records —
but a workflow that goes red on routine merges trains a reader to ignore it, which is precisely how
`BUG-27`'s outage survived six days. A gate nobody trusts is not a gate.

## Options

- `allow_empty: true` on the `PrepareRelease` step. Documented by knope for this exact case. Makes
  the step a no-op instead of an error, and the subsequent commit/push steps then have nothing to
  commit — so they need handling too.
- A guard step that checks for pending fragments and exits the job early, **logging that it skipped
  and why**. More code, but the skip is visible rather than inferred from a green tick.

The second is closer to this repo's own "no silent no-op" rule: a workflow that is green because it
correctly did nothing should say so.

## References

- BUG-27 -- removed `continue-on-error` from this step; this is the cost of that, and the right
  trade, but not a finished one.
- BUG-28 -- the release outage `BUG-27`'s `continue-on-error` had been hiding.
- SPEC-20 -- carries the false claim quoted above; needs correcting alongside the fix.
- ADR-45 -- the two-step release flow this step belongs to.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial record, `Status: Open`. **Why:** found by reading the run history while shipping `v0.2.1`, not by any alert -- the red run sat unexamined because nothing distinguishes "failed for a real reason" from "correctly had nothing to do" in this workflow's output. | **structural** |
