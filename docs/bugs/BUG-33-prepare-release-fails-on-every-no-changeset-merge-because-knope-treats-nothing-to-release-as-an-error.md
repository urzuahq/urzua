---
Stable-Id: 01M2MMFX94TCYK8M5H9KQ2VEMW
Status: Fixed
Found-in: 'checking why the prepare-release run that followed PR #37 was red -- knope exited 1 with `releases::no_release`, and #37 was a `no-changeset` PR with nothing to release'
Regression-test: 'the branch logic was exercised against a stubbed knope for all four cases -- success, no_release, a real failure, and a non-1 exit -- confirming only no_release is tolerated and other exit codes propagate unchanged; end to end, the merge of this fix is itself a no-changeset merge and must leave prepare-release green'
Realized-by: code:.github/workflows/prepare-release.yml
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

## The fix

Neither option as originally framed. `allow_empty: true` leaves the subsequent commit/push steps
with nothing to commit, moving the failure one step later. A pre-flight guard counting
`.changeset/*.md` files would be wrong too: knope also releases on Conventional Commits since the
last tag, so "no fragments" is not the same as "nothing to release" — the guard would skip a release
that was genuinely due.

Instead the step runs knope, captures its output, and tolerates **exactly one documented error**:

```sh
out=$(knope prepare-release --verbose 2>&1) && rc=0 || rc=$?
printf '%s\n' "$out"
[ "$rc" -eq 0 ] && exit 0
if printf '%s' "$out" | grep -q 'releases::no_release'; then
  echo "::notice::nothing to release -- no changeset fragment and no releasable commit since the last tag"
  exit 0
fi
echo "::error::knope prepare-release failed (exit $rc)"
exit "$rc"
```

This keeps `BUG-27`'s lesson intact. `continue-on-error` tolerates *every* failure, which is how a
six-day outage stayed invisible. This tolerates one, by name, and everything else fails with its
original exit code — including a future knope that stops emitting `releases::no_release`, which
fails loudly rather than being silently re-swallowed.

The skip emits a `::notice::` rather than passing quietly, so a green run that correctly did nothing
says so — `ADR-29`'s "never a silent skip", applied to the workflow rather than to a job.

Verified against a stubbed `knope` before shipping: success exits 0, `no_release` exits 0 with the
notice, a real failure exits 1, and a non-1 exit code propagates as itself rather than collapsing.

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
> | 2026-09-16 | Fixed. **Why:** both options this record originally proposed were wrong on inspection -- `allow_empty` moves the failure to the next step, and a fragment-count guard would skip a release genuinely due from Conventional Commits. Tolerating one error by name keeps `BUG-27`'s lesson (never swallow every failure) while removing the false alarm that trains a reader to ignore this workflow. | **substantive** |
> | 2026-09-16 | Initial record, `Status: Open`. **Why:** found by reading the run history while shipping `v0.2.1`, not by any alert -- the red run sat unexamined because nothing distinguishes "failed for a real reason" from "correctly had nothing to do" in this workflow's output. | **structural** |
