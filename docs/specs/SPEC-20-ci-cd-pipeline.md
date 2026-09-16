---
Stable-Id: 01M25Z28TWN477D6XASYT4TKW7
Status: Accepted
Date: 2026-09-10
Version: '0.3'
Author: beauwilliams
Subject: 'This repo''s own CI/CD pipeline -- gating, release cutting, and distribution, as one buildable reference.'
Implements: ADR-13, ADR-29, ADR-45
Parent: —
---
# SPEC-20 — CI/CD pipeline

## Purpose

This repo's own continuous integration and release pipeline, governed as one coherent feature area
(ADR-41's own "coherent feature area, decided editorially" principle applied here for the first
time to infrastructure rather than a record type): what gates a PR, how a release is cut, and how
binaries reach a user. Previously spread across several ADRs (`ADR-13`, `ADR-28`, `ADR-29`, `ADR-45`)
and three workflow files with no single document naming the whole shape. Not a schema spec — nothing
here is a `record_type`; this is process, realized entirely in `.github/workflows/*.yml` and
`knope.toml`.

## `ci.yml` — the PR/push gate

Deliberately unfiltered (no `branches:`/`paths:` restriction) — a filtered required check can fail
to report on a PR that doesn't touch the filtered paths, deadlocking a merge that requires it.

- **`rust` job**: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo build --release
  --locked`, `cargo test --locked`, then `urzua check docs/` against this repo's own corpus — the
  literal self-hosting exit criterion (`SPEC-1`). Checkout uses `fetch-depth: 0`:
  `embodiment.consistency`'s drift check (`ADR-32`) reads git blame/log history on locators, and a
  shallow checkout makes it silently unable to detect anything, not an error.
- **`changeset` job**: requires a `.changeset/*.md` fragment added in the diff, or the `no-changeset`
  label (`ADR-29`) — never a silent skip. Runs on `pull_request` only. The `pull_request:` trigger
  explicitly lists `[opened, synchronize, reopened, labeled, unlabeled]` (`BUG-15`) — the default
  three types alone leave this job evaluating a stale, pre-label event payload whenever the label is
  applied after the PR already exists (the common case for a fragment-less PR filed as an
  afterthought, and always the case for `prepare-release.yml`'s own automated PR below).

## `prepare-release.yml` / `publish-release.yml` — the two-step release (`ADR-45`)

- **`prepare-release.yml`** runs on every push to `main`, guarded against re-triggering on its own
  release-prep commit message. It force-pushes a `release` branch built from `main`, running knope's
  `prepare-release` workflow (`PrepareRelease` → `cargo check` → commit → push →
  `CreatePullRequest`), which compiles every accumulated `.changeset/*.md` fragment plus any
  Conventional Commits since the last tag into a version bump and `CHANGELOG.md` section. The job
  installs the pinned toolchain and a cargo cache first, because that `cargo check` needs both
  (`BUG-27`). **No `continue-on-error`**: swallowing the failure hid a six-day release outage once.
  The cost is that `knope prepare-release` treats nothing-to-release as an error, so **every
  `no-changeset` merge turns this workflow red** (`BUG-33`) — the guard skips only its own release
  commit, not a routine merge with no fragment. The failure direction is the safe one; a gate that
  is red on routine merges is still a gate nobody reads.
- The `cargo check` step exists because `PrepareRelease` only regex-bumps `Cargo.toml`'s
  version line — it has no knowledge of `Cargo.lock`, which independently records each workspace
  member's resolved version and goes stale the instant `Cargo.toml` changes without it (`BUG-14`,
  found on this mechanism's first real run: every `--locked` build/test/clippy invocation in this
  repo failed against the resulting PR). Not `--offline`: that needs an already-populated registry
  the release job doesn't have on a cold cache, and fails resolving external deps rather than doing
  the local-only refresh it was meant to (`BUG-27`). Not `--locked` either — the lockfile is stale by
  construction at that point, which is the whole reason the step exists. The committed `Cargo.lock`
  still pins every external version, so the check rewrites workspace members and nothing else.
- The resulting PR **is** the reviewable release artifact — the actual version bump and compiled
  changelog, visible as a real diff, not trusted sight-unseen inside a single dispatched job. Covered
  by the `no-changeset` label: it consumes fragments, it doesn't add one.
- **`publish-release.yml`** runs when that specific PR (head branch `release`) merges into `main`,
  and owns the entire publish — tag, GitHub Release, binaries, upload. Detailed below.

## `publish-release.yml` — the whole publish, in one workflow

Four jobs in sequence. **One workflow rather than two deliberately** (`BUG-28`): the tag is pushed
with `GITHUB_TOKEN`, and GitHub never triggers a workflow from a `GITHUB_TOKEN` event, so a separate
tag-triggered build workflow silently never runs — publishing a release with no binaries and no
error. `release.yml` previously held the last three jobs and is deleted.

- **`verify-ci`**: refuses to publish unless the `ci` check-run concluded `success` for the tree
  being released. Checks the release PR's **head sha**, not the merge commit, because `ci` on the
  merge commit races this workflow. It does **not** work because the head "gated the merge" — `main`
  has no branch protection, so nothing gates any merge, and the release PR is mergeable with no
  checks at all. In practice this gate asks whether a human approved the held `ci` run: merging
  before that approval publishes nothing and leaves `main` bumped with no tag.
- **`release`**: runs knope's `release` workflow (just the `Release` step) — tags the now-current
  version and creates the GitHub Release, with the compiled changelog as its notes since `[github]`
  config is present. Outputs the resolved tag, read from the same anchored `rust/Cargo.toml` line
  `knope.toml` itself bumps.
- **`build`**: cross-compiles for `aarch64-apple-darwin`, `x86_64-apple-darwin` (both from the same
  `macos-14` arm64 runner — a dedicated Intel runner queues indefinitely as GitHub winds down that
  capacity), and `x86_64-unknown-linux-gnu`. Checks out the **tag**, not a branch head, so a push
  landing on `main` mid-publish cannot produce binaries that disagree with their release.
- **`upload`**: `gh release upload` against the release `knope release` already created — never
  `gh release create`, which would collide with an existing release object. Then **asserts the
  result**: reads the release back and fails if fewer than three archives are attached. A release
  object existing is not the same as a release being installable.
- GitHub Release binaries only (`ADR-13`) — not published to crates.io; `Cargo.lock` is committed for
  this reason (a binary, not a library other crates depend on).

**Known manual steps — two, not one.** Both stem from `GITHUB_TOKEN` events not triggering
workflows, the same cause as `BUG-28`:

1. The release PR's `ci` run is held at `action_required` until a human approves it.
2. Approving it runs the **stale `opened` payload**, captured before knope's own labelling step, so
   the changeset job runs when it should skip and fails. A human must remove and re-add the
   `no-changeset` label to fire a `labeled` event from a real actor (`BUG-34`). `BUG-15`'s
   `labeled`/`unlabeled` trigger types exist for exactly this and are inert here, because the label
   is applied with `GITHUB_TOKEN`.

Neither step is discoverable from the failure: the job asks for a label that is already present.
`RFC-30` proposes removing both by acting as a GitHub App.

## `knope.toml` / `.changeset/*.md` — the fragment format (`ADR-29`)

- `[package].versioned_files` is an explicit, anchored-regex list (`(?m)^version = "..."$` for
  `rust/Cargo.toml`) — an unanchored pattern would also match every `{ version = "...", ... }`
  dependency pin in `[workspace.dependencies]` and `rust-version` itself, silently corrupting them on
  the next bump (verified against a live dry run before trusting it).
- `[github]` config (owner/repo) is required for `CreatePullRequest` and `Release` to reach the
  GitHub API — added by `ADR-45`, absent under the original single-dispatched-job design.
  `[[workflows]]` declares two named workflows, `prepare-release` and `release`, invoked as `knope
  prepare-release` / `knope release` respectively.
- A fragment (`.changeset/*.md`) is real changesets format: `default: major | minor | patch`
  frontmatter plus a Markdown body, one per user-visible PR. On a pre-`1.0.0` package, a `major`
  fragment bumps the minor version position per standard semver pre-1.0 convention (knope's default),
  never jumps straight to `1.0.0` on its own.
- A `README.md` with no changesets frontmatter inside `.changeset/` fails the release step outright
  if knope tries to parse it as a fragment — caught in a dry run before it shipped (`ADR-29`).

## What's deliberately not built

- **A hosted knope GitHub App** (`bot.releases`) — knope offers this as an alternative to running the
  CLI directly in a workflow; not adopted here, since the existing pinned-checksum CLI install
  already works and adding a hosted app is a new external dependency with no evidenced need.
- **Per-workspace-member versioning** — one version covers the whole product (`[package]`, not
  Cargo-workspace auto-detection); a multi-package release is not a shape this repo's binary-only
  distribution model needs.
- **crates.io publishing** — `ADR-13`'s own decision; GitHub Release binaries only.

## References

- ADR-13 — GitHub Release binaries, not crates.io; the reason `publish-release.yml` cross-compiles
  at all and `Cargo.lock` is committed.
- BUG-28 — why the build and upload jobs live in `publish-release.yml` rather than a separate
  tag-triggered workflow.
- ADR-28 — the original rejection of a Node-based changelog tool; `ADR-29`'s reasoning for choosing
  `knope` still stands on this.
- ADR-29 — changesets via `knope`; the fragment format and versioned-files config, superseded in its
  single-dispatched-job sequencing only by `ADR-45`.
- ADR-32 — the git-blame drift detection `ci.yml`'s `fetch-depth: 0` exists to support.
- ADR-45 — the two-step `prepare-release`/`publish-release` split this spec documents.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial spec. **Why:** the CI/CD pipeline had real decisions spread across four ADRs and three workflow files with no single buildable reference — found live while building `ADR-45`'s two-step release flow, the same "coherent feature area, decided editorially" gap `ADR-41` already named for record types, here applied to infrastructure for the first time. | **structural** |
> | 2026-09-10 | Documented two fixes found on this mechanism's first real run, same-day as `ADR-45` merged: `BUG-14` (`prepare-release`'s missing `Cargo.lock` regeneration) and `BUG-15` (`ci.yml`'s changeset gate missing `labeled`/`unlabeled` trigger types). **Why:** this spec's own standing purpose is to stay the accurate, buildable reference — leaving it describing the pre-fix mechanism the same day it changed would be the exact doc-drift gap this project's own `AGENTS.md` now explicitly guards against. | **substantive** |
> | 2026-09-16 | Corrected three claims, all disproved by the `v0.2.1` run: the `no-changeset` no-op (`BUG-33`), `verify-ci`'s "it gated the merge" justification (`main` is unprotected), and the manual gate being one step rather than two (`BUG-34`). **Why:** this spec's standing purpose is to be the accurate, buildable reference, and each of these read as settled while being false -- the kind of claim that survives precisely because it is written down. | **substantive** |
> | 2026-09-16 | Rewrote the release section: `publish-release.yml` now owns the whole publish (tag, release, binaries, upload) and `release.yml` is deleted. Documented the previously-undocumented manual approval the release PR's own `ci` run requires. **Why:** `BUG-28` -- the tag is bot-pushed, GitHub never triggers a workflow from a `GITHUB_TOKEN` event, so the separate tag-triggered build workflow silently never ran and `v0.2.0` published with zero binaries. This spec described a coupling that did not exist. | **substantive** |
