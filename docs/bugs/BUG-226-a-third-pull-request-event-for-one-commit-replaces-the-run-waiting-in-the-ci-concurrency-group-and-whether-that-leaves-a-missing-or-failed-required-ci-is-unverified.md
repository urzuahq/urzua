---
Stable-Id: 01M861TZ8GEBKGHVBC4SQNJE0F
Status: Open
Found-in: 'BUG-222, Observation (2026-11-24) and the caveat before it: with cancel-in-progress false for every event but a new commit, a third event can still replace the waiting run'
Realized-by:
---
# 226 — A third pull_request event for one commit replaces the run waiting in the ci concurrency group, and whether that leaves a missing or failed required ci is unverified

## What was wrong

`.github/workflows/ci.yml` sets `cancel-in-progress` true only for a `synchronize` event on a pull request, so
a second event for one commit waits behind the run in progress instead of cancelling it. A third event for the
same commit, arriving while one run is in progress and one is waiting, replaces the waiting run: GitHub keeps
at most one pending run per concurrency group, and the replaced run is cancelled before it starts.

Whether that leaves a missing or failed required `ci` check is unverified. `BUG-222` records the caveat, and
its `Observation (2026-11-24)` section records one unreproduced case from a sibling repository: the replaced
run showed `cancelled` with zero jobs and posted no `ci` check, and the head commit carried one green `ci`.
That is a single observation. It does not show whether the missing check blocks a required-check ruleset.

Possible remedies, none chosen and none verified on GitHub:

- a group key that includes the event action for non-`synchronize` runs, so the third event lands in a group
  of its own and replaces nothing;
- GitHub's concurrency `queue` property, if it exists and queues instead of replacing the pending run
  (unverified).

## Why nothing caught it

The replacement of a pending run is GitHub's scheduler, not a script in this repository, so no test can
exercise it. The fix for `BUG-222` was applied with this case still open, and nothing recorded it until now.

## References

- `BUG-222` (the caveat and `Observation (2026-11-24)`), `docs/specs/SPEC-20-ci-cd-pipeline.md`,
  `.github/workflows/ci.yml` (the `concurrency` block).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
