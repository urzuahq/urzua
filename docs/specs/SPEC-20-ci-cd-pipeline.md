---
Stable-Id: 01M25Z28TWN477D6XASYT4TKW7
Status: Accepted
Date: 2026-09-10
Version: '0.2'
Author: '@beauwilliams'
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
  `prepare-release` workflow (`PrepareRelease` → `cargo check --offline` → commit → push →
  `CreatePullRequest`), which compiles every accumulated `.changeset/*.md` fragment plus any
  Conventional Commits since the last tag into a version bump and `CHANGELOG.md` section.
  `continue-on-error: true` on the knope invocation: when there's nothing to release, the first step
  to run out of fragments fails fast and the rest of the job never executes, without turning a
  routine docs-only merge into a red check.
- The `cargo check --offline` step exists because `PrepareRelease` only regex-bumps `Cargo.toml`'s
  version line — it has no knowledge of `Cargo.lock`, which independently records each workspace
  member's resolved version and goes stale the instant `Cargo.toml` changes without it (`BUG-14`,
  found on this mechanism's first real run: every `--locked` build/test/clippy invocation in this
  repo failed against the resulting PR). `--offline` refreshes only the local path-dependency
  versions already resolved — no external dependency pin changes, no network access.
- The resulting PR **is** the reviewable release artifact — the actual version bump and compiled
  changelog, visible as a real diff, not trusted sight-unseen inside a single dispatched job. Covered
  by the `no-changeset` label: it consumes fragments, it doesn't add one.
- **`publish-release.yml`** runs when that specific PR (head branch `release`) merges into `main`,
  and runs knope's `release` workflow (just the `Release` step) — tags the now-current version and
  creates the GitHub Release (with the compiled changelog as its notes, since `[github]` config is
  present), which fires `release.yml` below.

## `release.yml` — cross-compiled binaries, GitHub Release

Tag-triggered (`push: tags: ["v*"]`), untouched by `ADR-45` except for one consequence:

- **`verify-ci`**: refuses to build unless the `ci` workflow's check-run concluded `success` for the
  exact tagged commit — a tag pushed against a red or unevaluated commit must not produce a release.
- **`build`**: cross-compiles for `aarch64-apple-darwin`, `x86_64-apple-darwin` (both from the same
  `macos-14` arm64 runner — a dedicated Intel runner queues indefinitely as GitHub winds down that
  capacity), and `x86_64-unknown-linux-gnu`.
- **`publish-release`**: uploads the built archives to the GitHub Release. Since `ADR-45`, this is
  `gh release upload` against the release `publish-release.yml`'s `knope release` step already
  created — previously `gh release create ... --generate-notes`, which would now collide with a
  release object that already exists by the time this job runs.
- GitHub Release binaries only (`ADR-13`) — not published to crates.io; `Cargo.lock` is committed for
  this reason (a binary, not a library other crates depend on).

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

- ADR-13 — GitHub Release binaries, not crates.io; the reason `release.yml` exists in its current
  shape and `Cargo.lock` is committed.
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
