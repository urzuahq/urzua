---
Stable-Id: 01M6R3NK4GWRMJ1PXFNHSKT27V
Status: Superseded
Embodiment: Implemented
Realized-by: code:.github/workflows/ci.yml@sha256:a2b83039753098dd01df37de8b7a526422e290f9fb11ed400ced6a92849ca4ea
Date: 2026-09-06
Accepted-on: 2026-09-06
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: ADR-29
Derives-from: —
---
# 28 — Changelog lockstep, verified in CI; never auto-generated

## Context

`CHANGELOG.md` has stated its own purpose since it was created: "Written for the person installing
it — what changed for you — not a commit dump." It has been stale since `v0.1.0` while real,
user-visible commands (`explain`, `graph`, `fix --apply`, `migrate ids`, `migrate schema --report`,
now `new`) shipped with nothing recorded about them. Nothing in CI checks that the file is kept
current, so staleness is silent until a release, at which point it's a scramble to reconstruct what
actually shipped since the last tag.

The two shapes considered: auto-generate changelog prose from commit messages or PR titles (several
public tools do this), or verify a human wrote an entry whenever it actually matters. Generation
needs no human judgment to run, but a commit message is written for someone reviewing this PR, not
someone deciding whether to upgrade — it routinely says *what* changed, never *why it matters to an
installer*, which is exactly the distinction `CHANGELOG.md`'s own stated purpose draws.

## Decision

In the context of a changelog whose entire value is judgment a commit message doesn't carry, facing
staleness that nothing catches today, we decided: **CI verifies lockstep, it never generates
prose.** A job compares `rust/Cargo.toml`'s workspace version against the PR's base branch. If the
version is unchanged, the check passes trivially — most PRs don't ship a release. If the version
changed, `CHANGELOG.md` must contain a `## [<new-version>]` heading, or the job fails with exactly
which version is missing. What goes under that heading is still entirely hand-written, still
reviewed like any other prose change, and still following the file's existing format (loosely
[Keep a Changelog](https://keepachangelog.com/)).

## Reversibility

Cheap to add, cheap to remove: one CI job with no effect on anything that doesn't bump the version
field. Fails closed only on the one condition it exists to catch.

## Consequences

- A version bump with no changelog entry now fails CI instead of shipping silently stale.
- A PR that doesn't touch the version number is entirely unaffected — this is not a "changelog
  required on every PR" gate, which would either get bypassed constantly or force a changelog entry
  for changes with nothing user-visible to say.
- `CHANGELOG.md` itself is not retroactively backfilled by this ADR; that is separate, explicit work
  (tracked as a known gap, not silently deferred) needed before the next version bump ships clean.
- No dependency on a Node-based changelog tool — the check is a `bash` step in the existing CI job,
  consistent with this repo's Rust-only, Makefile-driven CI stance.
- **Superseded by ADR-29**: lockstep verification caught staleness but still left the changelog
  entry itself hand-reconstructed at release time, from memory, after the motivating PR had already
  merged. ADR-29 moves the note-writing to when the PR is written (a per-PR changeset fragment)
  and keeps the same Rust-only constraint this ADR named, via `knope` rather than a Node tool. The
  `changelog` CI job this ADR describes no longer exists; ADR-29's `changeset` job replaces it.

## References

- `CHANGELOG.md` — the file's own stated purpose, which this ADR enforces rather than restates.
- `.github/workflows/ci.yml` — the `changelog` job.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Pinned `.github/workflows/ci.yml` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-verified for a Superseded record: the only change to ci.yml is an unrelated changeset-required fix. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/ci.yml` cc0baa44652e -> 3d22f5501990. Reason: Read the diff of each pinned span since the old pin and the claim still holds. .github/workflows/ci.yml: gained the edited and merge_group triggers and a push to main no longer cancels a run in progress, the changelog check the ADR describes is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/ci.yml` 3d22f5501990 -> a2b830397530. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. ci.yml: the required-check job now passes the toJSON of every needed job to .github/scripts/ci-gate.sh (a failure, a cancellation and a skip all fail it, only changeset may skip on a pull request) where it counted failures and cancellations inline, and the jobs and triggers the record names are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
