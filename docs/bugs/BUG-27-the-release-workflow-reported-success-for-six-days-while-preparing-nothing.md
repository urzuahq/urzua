---
Stable-Id: 01M2M6K1FPPSNSENWWZFRGG4T1
Status: Fixed
Found-in: 'closing out a merge queue -- six PRs merged to main, and the release PR they should have regenerated was still showing a six-day-old diff while every prepare-release run reported success'
Regression-test: 'not yet written -- see Why nothing caught it; the honest gap is that no test exercises this workflow, and the fix is a workflow/config change a unit test cannot reach'
Realized-by: code:.github/workflows/prepare-release.yml, code:knope.toml
---
# 27 — The release workflow reported success for six days while preparing nothing

## What was wrong

`prepare-release.yml` ran on every push to `main` and reported **success every time**. It had
prepared nothing since 2026-09-10. The release branch sat eleven commits behind `main`, holding a
six-day-old commit, while twenty changesets accumulated unreleased.

The chain, in order:

1. `BUG-0014`'s fix added a `cargo check --offline --manifest-path rust/Cargo.toml --workspace` step
   to `knope.toml`'s `prepare-release` workflow, so the lockfile's workspace-member versions get
   regenerated after `PrepareRelease` bumps `rust/Cargo.toml`.
2. `prepare-release.yml` has **no Rust toolchain step and no cargo registry cache** — unlike
   `ci.yml`, which does `rustup show` plus `Swatinem/rust-cache`. It was written before `knope.toml`
   had a cargo step in it, and nobody revisited the job when one was added.
3. With no populated registry, `--offline` cannot resolve any external dependency. The step failed
   with `error: no matching package named 'serde' found` / `note: offline mode (via '--offline') can
   sometimes cause surprising resolution failures`, cargo exited non-zero, and knope aborted the
   whole workflow at that step — before the commit, the push, and the `CreatePullRequest` that were
   supposed to follow.
4. The step carried `continue-on-error: true`, so the job went green anyway.

`--offline` was chosen deliberately and the reasoning was recorded in `knope.toml`: the step "only
needs to refresh the local path-dependency versions already resolved, never touch external
dependency pins or hit the network." The intent was right; the flag assumed a populated registry
that this particular job never had.

## Why nothing caught it

**The workflow's only signal was its own exit status, and that was configured to lie.** Every
prepare-release run showed a green check. Nothing compared the release branch against `main`,
nothing asserted a PR had been updated, nothing counted unreleased changesets. The one place the
truth was visible was 1,400 lines into a log nobody reads when the badge is green.

This is the failure class this project's own `SPEC-0001` calls non-negotiable — *"a check that finds
nothing must be distinguishable from a check that ran on nothing"* — applied to `urzua check`'s
findings but never to the repository's own CI. `urzua doctor`'s `ci-wired` check asks whether a
workflow *invokes* the tool; no check asks whether a workflow that ran actually *did anything*.

`continue-on-error: true` was added for a real reason: a routine push with no unreleased changesets
makes `PrepareRelease` fail, and the job shouldn't go red for that. But the workflow already has an
`if:` guard for the one expected no-op case, so the flag was redundant belt-and-braces that
converted every real failure into silence.

## The fix

1. `prepare-release.yml` gains `rustup show` and `Swatinem/rust-cache` before the knope step, so the
   job has the toolchain and registry `knope.toml`'s cargo step needs.
2. `knope.toml` drops `--offline`. Not `--locked` either: the lockfile is stale by construction at
   that point — regenerating it is the entire purpose of the step. The existing `Cargo.lock` still
   pins every external version, so the check rewrites workspace members only. Verified directly:
   bumping `rust/Cargo.toml` to `0.2.0` and running the command produced a five-line lockfile diff,
   all five workspace members, no external pin touched.
3. `continue-on-error: true` removed. The `if:` guard covers the expected no-op; anything else
   reaching that step is real and should be red.

## References

- BUG-0014 — added the cargo step whose flag this bug is about; its own fix was correct in intent
  and untested against the job it runs in.
- ADR-0045 — the two-step release mechanism this workflow implements.
- SPEC-0001 — the no-silent-no-op rule, stated for `check` and not yet applied to this repository's
  own workflows.
- `.github/workflows/ci.yml` — the toolchain setup `prepare-release.yml` was missing.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change. **Why:** found while closing out a six-PR merge queue -- the release PR should have regenerated on every merge and hadn't since it was opened. The diagnosis is worth keeping even though the fix is small: an untested workflow step, a flag whose precondition the job never satisfied, and an error-swallowing setting combined into six days of confident green. | **substantive** |
