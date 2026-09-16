---
Stable-Id: 01M2MQK9NYKNJV8C0ZVW411T4W
Status: Accepted
Embodiment: Implemented
Realized-by: code:knope.toml
Date: 2026-09-16
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-29
---
# 48 — Changesets are the only release trigger; Conventional Commits are ignored

## Context

`ADR-29` chose changesets: a `.changeset/*.md` fragment per user-visible PR, compiled by knope into a
version bump and `CHANGELOG.md` section. `ci.yml` enforces it — every PR adds a fragment or carries
the `no-changeset` label, never a silent skip.

knope also reads Conventional Commits since the last tag, and `knope.toml` declares
`type = "PrepareRelease"` with no options, so that behaviour is knope's default rather than anything
decided here. `ADR-29` mentions it only descriptively.

Two signals, one decided and one inherited, and nothing compares them. `BUG-35` is what that
produced: a CI-only PR labelled `no-changeset` whose single commit read
`fix(release): ...`, which knope turned into a `0.2.2` release whose changelog is one sentence about
this repo's CI.

Two facts sharpen the choice.

**This repo does not write Conventional Commits.** One of the last forty commits on `main` carries a
releasable prefix, and it is the mistaken one. The convention here is descriptive subjects, which
knope correctly ignores. Turning the feature off changes nothing that has ever happened.

**The signal is the wrong shape regardless of discipline.** A Conventional Commit infers "releasable"
from a commit message, with no knowledge of which tree changed. This repo tracks 225 files under
`docs/` and 39 under `rust/`, plus `ts/`, `platform/` and `scripts/`, while `[package]` versions only
`rust/Cargo.toml` and the published archive carries the binary, `LICENSE` and `README.md`. A `fix:`
commit touching `docs/` would bump the binary's version. No amount of careful typing fixes a signal
that cannot see paths.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Leave both signals active | No change | Two sources of truth that can contradict; `BUG-35` is the result, and it recurs whenever anyone types a prefix out of habit |
| **`ignore_conventional_commits = true`** | One signal; the contradiction becomes unrepresentable; costs nothing, since no release has ever been computed from a prefix | A genuine user-facing change with a `fix:` commit and no fragment would not ship — mitigated below |
| A CI check that the label and the commit prefix agree | Keeps both mechanisms | Three things to maintain instead of two, and it must re-derive knope's own semantics in shell to know which prefixes are releasable |
| Adopt Conventional Commits properly and drop changesets | One signal, the other way | Loses the reviewable fragment `ADR-29` chose it for, and still cannot see which tree changed |

## Decision

In the context of a repository where only one of several trees produces the released artifact, and
where a decided mechanism runs alongside an inherited default that can contradict it, we decided:
**a `.changeset/*.md` fragment is the only way to compute a release.**
`ignore_conventional_commits = true` on the `PrepareRelease` step.

The fragment is the right granularity because it is a human declaring *this changes what an installer
receives* — a judgement no commit-message convention can encode, because the judgement is about the
published artifact rather than about the diff.

This is the same shape as `ADR-46` removing `serde(flatten)` and `ADR-47` removing the cross-workflow
query: delete the mechanism that makes the defect possible, rather than adding a check that the
defect has not occurred.

## Reversibility

One line. Remove `ignore_conventional_commits = true` and both signals are active again. No data
format changes; `.changeset/*.md` fragments and `CHANGELOG.md` are unaffected.

## Consequences

- **A user-facing change with no fragment will not ship a release.** Acceptable, because `ci.yml`
  already requires every PR to add a fragment or be explicitly labelled `no-changeset` — you cannot
  merge a user-facing change without someone declaring one way or the other. The commit prefix was a
  second, silent vote on a question already being asked out loud.
- **Commit-message style is now free.** `fix:`, `feat:`, or a plain descriptive subject all behave
  identically for release purposes, so nobody has to remember a rule about it. This matters more than
  it sounds: the rule "don't use Conventional prefixes" is exactly the kind `RFC-7` rules unenforced,
  and `BUG-35` was written by an agent that had read this repo's guidance.
- **`prepare-release` will now take the no-release path more often** — every merge without a
  fragment. `BUG-33` already made that a green run with a `::notice::` rather than a red one.
- **If this repo ever versions more than `rust/`**, this decision needs revisiting alongside knope's
  multi-package configuration; `SPEC-20` records per-workspace-member versioning as deliberately not
  built.

## References

- ADR-29 -- chose changesets; this removes the undecided second trigger that ran beside it.
- BUG-35 -- the observed instance.
- BUG-33 -- the fix whose merge produced it, and whose own reasoning (a fragment count is not
  "nothing to release") this decision makes moot by removing the other source.
- ADR-46, ADR-47 -- the same move: remove the mechanism rather than check the outcome.
- RFC-7 -- why "just don't type that prefix" is not a fix.
- SPEC-20 -- the CI/CD reference this changes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial decision. **Why:** an undecided default sat beside a decided mechanism and contradicted it, and the first commit to expose that cut an unintended release. Removing the second trigger costs nothing here -- no release has ever been computed from a commit prefix except the mistaken one. | **structural** |
