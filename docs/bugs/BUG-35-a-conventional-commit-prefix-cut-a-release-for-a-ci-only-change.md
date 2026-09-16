---
Stable-Id: 01M2MQK99NNTQZ6V91PJEWE5GE
Status: Fixed
Found-in: 'merging BUG-33''s own fix (#41) -- the merge was meant to exercise the no-release path and instead opened a 0.2.2 release PR'
Regression-test: 'knope is configured to ignore Conventional Commits entirely, so a commit prefix can no longer compute a release -- the trigger is removed rather than checked; observable on the next no-changeset merge, which must leave prepare-release green with no release PR'
Realized-by: code:knope.toml
---
# 35 — A Conventional Commit prefix cut a release for a CI-only change

## What was wrong

`#41` was a CI-only change: it altered how `prepare-release.yml` handles knope's exit status and
touched nothing an installer receives. It carried the `no-changeset` label, which is this repo's
declared way of saying a change has nothing to tell an installer (`ADR-29`).

It had one commit, so GitHub used that commit's message for the squash:

```
fix(release): tolerate knope's no_release by name, not every failure (#41)
```

knope reads Conventional Commits in addition to `.changeset/*.md` fragments. `fix` means a patch
release, so it computed `0.2.2` and opened a release PR whose entire changelog reads:

> tolerate knope's no_release by name, not every failure (#41)

A sentence about this repo's CI, in a document written for people installing the tool.

**The same PR asserted both things at once.** The `no-changeset` label said "nothing to tell an
installer"; the commit prefix said "user-visible patch release". Nothing compared them.

## Why nothing caught it

Two independent signals decide whether a change is releasable, and only one was ever decided.
`ADR-29` chose changesets and describes Conventional Commits in passing as something knope also
reads. `knope.toml` declares `type = "PrepareRelease"` with no options, so
`ignore_conventional_commits: false` is knope's own default, inherited rather than chosen.

It had not fired before because this repo does not write Conventional Commits. Of the last forty
commits on `main`, **one** carries a releasable prefix — the one above. The convention here is
descriptive subjects (`File RFC-28 + BUG-26 ...`, `ADR-45: two-step release via knope ...`), which
knope correctly ignores. The trap was live from the day `ADR-29` landed and nobody had walked into
it.

## The deeper mismatch

Even written perfectly, the signal is the wrong shape for this repository. A Conventional Commit
infers "releasable" from a commit *message*, with no knowledge of which tree changed. This repo
tracks 225 files under `docs/`, 39 under `rust/`, plus `ts/`, `platform/` and `scripts/` — and
`[package]` versions only `rust/Cargo.toml`, with the published archive containing the `urzua`
binary, `LICENSE` and `README.md`. Nothing else reaches an installer.

So a `fix:` commit touching `docs/` or `scripts/` bumps the Rust binary's version. That is not a
discipline failure that better commit messages would prevent; the signal has no path-awareness to
give.

## The fix

`ignore_conventional_commits = true` on the `PrepareRelease` step. A `.changeset/*.md` fragment
becomes the only way to compute a release. The contradiction stops being something to check and
becomes unrepresentable — one signal, deliberately written by a human declaring that the change
affects what an installer receives.

Nothing that has ever happened in this repo changes: no release has been computed from a commit
prefix except the mistaken one this record exists for.

## References

- ADR-48 -- the decision, including the rejected alternative of checking the two signals agree.
- ADR-29 -- chose changesets; mentions Conventional Commits descriptively without deciding them.
- BUG-33 -- the fix whose own merge produced this; also the reason a fragment-count guard was
  rejected there, which this instance proves correct.
- SPEC-20 -- the CI/CD reference.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change. **Why:** found by merging another fix and watching the result rather than assuming it -- the merge was expected to exercise `BUG-33`'s no-release path and did something else entirely. Worth recording that the trap had existed unsprung since `ADR-29`, and that the first commit to spring it was written by an agent working on the release pipeline. | **substantive** |
