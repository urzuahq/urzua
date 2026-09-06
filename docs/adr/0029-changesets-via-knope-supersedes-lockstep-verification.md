# 0029 — Changesets via `knope`, superseding lockstep verification

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:knope.toml, code:.github/workflows/ci.yml, code:.github/workflows/prepare-release.yml
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: ADR-0028
> Derives-from: —

## Context

ADR-0028 shipped CI verification only: a version bump had to carry a matching `CHANGELOG.md`
heading, but the heading's content was still hand-written at release time, from memory, after the
PR that motivated it had already merged. That is exactly the failure mode `CHANGELOG.md`'s own
stated purpose warns about — a commit dump reconstructed later is not the same as a note written by
someone who had the context fresh.

The real fix is to capture that note *when the PR is written*, then let tooling assemble it later.
That is what the JS `changesets` ecosystem does: a markdown fragment per PR, consumed at release
time to bump the version and compile the changelog. The mechanism is good; the concrete tool is not
— this repo already turned down a Node-based changelog tool once (ADR-0028's own rejection of
`changesets/action`), and nothing about that reasoning changed. **`knope`** is the same
fragment-per-PR model as a Rust binary: its `changesets` crate implements the identical markdown
format, `knope document-change` authors a fragment, and `knope release` consumes every fragment plus
any Conventional Commits since the last tag to bump the version and compile `CHANGELOG.md`.

Two real compatibility risks surfaced verifying this against the actual repo, not just the docs,
before trusting it:

- Knope's Cargo-workspace auto-detection (used only when no `knope.toml` exists) treats every
  workspace member as its own versioned package — wrong here, where one version covers the whole
  product. An explicit `[package]` table sidesteps it entirely.
- An unanchored regex against `version = "..."` matches every dependency pinned as `{ version =
  "...", ... }` in `[workspace.dependencies]` (`serde`, `clap`) and `rust-version` itself, silently
  corrupting them on the next bump. A dry run against a scratch copy of this repo reproduced the
  corruption before it was fixed — the pattern must be anchored to the start of a line
  (`(?m)^version = "..."$`), which only `[workspace.package]`'s own `version` line satisfies.

## Decision

In the context of a changelog that needs a human's in-the-moment note rather than a reconstruction,
facing the same Node-dependency tradeoff ADR-0028 already turned down, we decided: **`knope` manages
changesets; CI gates on a fragment existing; a maintainer runs the actual release by hand.**

- `.changeset/*.md` fragments, one per user-visible PR, in the real changesets format (`knope.toml`
  §`[package]`; the format is documented in `CONTRIBUTING.md`, not inside `.changeset/` itself --
  knope tries to parse every `.md` file there as a fragment, and a `README.md` with no frontmatter
  fails the release step outright, caught in a dry run before it shipped).
- CI's PR gate changes from "does the changelog have this version" (ADR-0028) to "does this PR carry
  a fragment" — pass via a `.changeset/*.md` file in the diff, or an explicit `no-changeset` label
  for PRs with nothing to tell an installer. Never a silent skip: a PR with neither fails.
- Cutting a release stays a deliberate, maintainer-triggered act (`workflow_dispatch`), matching how
  every commit in this repo's actual history has landed — a direct, reviewed push, never an
  unattended action on every merge to main. `knope release` bumps the version, compiles
  `CHANGELOG.md` from the accumulated fragments, commits, tags, and pushes; the existing tag-
  triggered `release.yml` (cross-compiled binaries, GitHub Release) picks up from there unchanged.

## Reversibility

Additive over ADR-0028, not a rewrite of anything downstream: the tag-triggered release pipeline is
untouched, and `CHANGELOG.md`'s own format/purpose is unchanged, just populated automatically.
Dropping `knope` later means going back to hand-editing the file `knope release` would have written —
no format lock-in, since the output is plain Markdown.

## Consequences

- `CHANGELOG.md` gains a new heading style going forward (`## x.y.z (date)`, `knope`'s own format,
  not the `## [x.y.z] — date` bracket style the `v0.1.0` entry used) — the existing entry is
  reformatted to match in the same change, so the file doesn't carry two conventions indefinitely.
- A PR touching only CI config, internal refactors, or docs can skip a fragment via the
  `no-changeset` label rather than writing an empty one — the escape hatch real `changesets/action`
  offers, reproduced without installing it.
- `prepare-release.yml`'s job is the one workflow in this repo with `contents: write` and a git
  identity configured for a real push — a narrow, explicit exception to the read-only default,
  gated behind `workflow_dispatch` (a maintainer's deliberate action), never a trigger on merge.

## References

- ADR-0028 — the verification-only decision this supersedes.
- `.changeset/README.md` — the fragment format, for contributors.
- `knope.toml`, `.github/workflows/ci.yml`, `.github/workflows/prepare-release.yml`.
