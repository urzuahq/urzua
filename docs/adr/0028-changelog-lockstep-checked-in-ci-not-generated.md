# 0028 — Changelog lockstep, verified in CI; never auto-generated

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:.github/workflows/ci.yml
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: —

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

## References

- `CHANGELOG.md` — the file's own stated purpose, which this ADR enforces rather than restates.
- `.github/workflows/ci.yml` — the `changelog` job.
