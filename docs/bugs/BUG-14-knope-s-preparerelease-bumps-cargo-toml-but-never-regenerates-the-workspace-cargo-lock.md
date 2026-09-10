---
Stable-Id: 01M265JQ75D573RGZQGVZ8YT3Y
Status: Fixed
Found-in: 'hit live on the very first real run of ADR-45''s prepare-release workflow, merging PR #23 into main: PR #24 (the actual "chore: prepare release 0.2.0" PR it opened) failed `rust (fmt, clippy, test, build)` outright -- `cargo clippy --workspace --all-targets --locked` refused to run, "the lock file needs to be updated but --locked was passed"'
Regression-test: 'no automated test -- verified by hand: `cargo check --offline --manifest-path rust/Cargo.toml --workspace` before vs. after the fix, confirming it produces exactly the five workspace-member version bumps and nothing else (no external dependency changes)'
---
# 14 — knope's PrepareRelease bumps Cargo.toml but never regenerates the workspace Cargo.lock

## What was wrong

`PrepareRelease` only touches `[package].versioned_files` -- for this repo, a single regex-anchored
line in `rust/Cargo.toml`. It has no knowledge of `Cargo.lock`, which independently records the
resolved version of every workspace member (`urzua-core`, `urzua-cli`, `urzua-io`, `urzua-id`,
`urzua-agdr`) as its own `[[package]]` entry. Bumping `Cargo.toml`'s `[workspace.package].version`
without updating `Cargo.lock` to match leaves the lockfile stale the instant the version changes --
and every build/test/clippy invocation in this repo's own CI and pre-push hook runs `--locked`
specifically to catch drift like this, so the very next command run against the bumped version
failed immediately, on the first real release PR this mechanism ever produced.

## Why nothing caught it

The `--dry-run` verification done before merging `ADR-45` proved the *content* of the version bump
and changelog compilation was correct, but `--dry-run` only prints what commands *would* run --
it never actually executes `cargo clippy --locked` or any other command that would have caught a
stale lockfile, because nothing in that verification pass ran the real build against the
hypothetical bumped state. The gap was invisible until a real (non-dry-run) execution actually
produced a commit with a changed `Cargo.toml` and an untouched `Cargo.lock` side by side.

## References

- `knope.toml` -- the `prepare-release` workflow, which now runs
  `cargo check --offline --manifest-path rust/Cargo.toml --workspace` immediately after
  `PrepareRelease` and stages the result, before committing.
- ADR-45 -- the two-step release flow this bug was found realizing for the first time.
- PR #24 -- the actual release PR this defect broke on its first real run.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Found and fixed same-day, on the first real run of `ADR-45`'s mechanism. `knope.toml`'s `prepare-release` workflow gains a `cargo check --offline` step (regenerates only the local workspace members' recorded versions, touches no external dependency, needs no network) plus an explicit `git add rust/Cargo.lock`, both before the release-prep commit. `Status: Fixed`. | **substantive** |
