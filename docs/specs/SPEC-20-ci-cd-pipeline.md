---
Stable-Id: 01M25Z28TWN477D6XASYT4TKW7
Status: Shipped
Embodiment: Verified
Realized-by: code:.github/workflows/ci.yml@sha256:3d22f5501990754b5b20fd4a05ebb02b6ed70013382eb19a4f838ffd49610036, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, code:.github/workflows/prepare-release.yml@sha256:0e4d28280d1a77819841035b7ed7da5ee570cf96e80875d2f1e8cc83d7596b8d, code:.github/workflows/publish-release.yml@sha256:530bdeeececf77737d4d162e8aba4c8685bcc73834140675ed8565e3a04aef25, code:.github/actions/install-knope/action.yml@sha256:ace7ea7ca992d45b2e53f32dbb852b36faca2b2d828e4e9ae378d3b54a1702ab, test:.github/scripts/knope-pin.test.sh@sha256:2c13f4fc3ca389b738f6f0a913e45696023f38236868bae27d5a9a352ad40a8d, test:.github/scripts/release-guard.test.sh@sha256:5fca6fe029d9bc486b023e4a401d0ae297f0e4e57ed9bb7a4f54fe62f9ceb82f, test:.github/scripts/release-invariants.test.sh@sha256:8fc2b5758338aef9953492f6d1977d9799c662f05589bf1d9064e6e5129b1381, test:.github/scripts/test-no-leak.test.sh@sha256:36c44d3cd4c56c6483e31d10e31ce44b29db039518446a3f305a1f1f4a2bce9d, test:.github/actions/urzua-gate/test.sh@sha256:2d9fb3612a1d978cc8d98b8a1ce0b6611eaf02502d0e31080a4eeef271e70702
Date: 2026-09-10
Version: '0.12'
Author: beauwilliams
Subject: 'This repo''s own CI/CD pipeline -- gating, release cutting, and distribution, as one buildable reference.'
Implements: ADR-13, ADR-45
Parent: —
---
# SPEC-20 — CI/CD pipeline

## Purpose

This repo's own continuous integration and release pipeline, governed as one coherent feature area
(ADR-41's own "coherent feature area, decided editorially" principle applied here for the first
time to infrastructure rather than a record type): what gates a PR, how a release is cut, and how
binaries reach a user. Previously spread across several ADRs (`ADR-13`, `ADR-28`, `ADR-29`, `ADR-45`)
and four workflow files with no single document naming the whole shape. Not a schema spec — nothing
here is a `record_type`; this is process, realized entirely in `.github/workflows/*.yml` and
`knope.toml`.

## `ci.yml` and `checks.yml` — the PR/push gate

`ci.yml` runs on `pull_request` (`opened`, `synchronize`, `reopened`, `labeled`, `unlabeled`, `edited`),
`merge_group`, `push` to `main` and `workflow_dispatch`. It is deliberately unfiltered (no
`branches:`/`paths:` restriction) — a filtered required check can fail to report on a PR that doesn't
touch the filtered paths, deadlocking a merge that requires it. Its concurrency group is per ref and
cancels a superseded run, except on a push to `main`, whose run always finishes.

The required check is the aggregate job **`ci`**. It `needs` every other job, runs `if: always()`, and
fails when any of them failed or was cancelled. It is the only name a ruleset should require; the jobs
nested inside a called workflow have unstable names (`ADR-95`).

- **`rust`** calls the reusable workflow `checks.yml` (`ADR-47`), with `pull-requests: write` so the gate
  can post its one updating comment. `publish-release.yml` calls the same workflow against the commit it is
  about to tag, so there is one definition of the gate rather than two that can drift. `checks.yml` runs,
  in order, with `fetch-depth: 0` (`embodiment.consistency`'s drift check (`ADR-32`) reads git
  blame/log history on locators, and a shallow checkout makes it silently unable to detect anything, not
  an error):
  - `cargo fmt --check`, `cargo clippy -- -D warnings`;
  - `cargo machete` and `cargo deny check`, pinned prebuilt binaries verified by checksum;
  - the pinned, checksummed knope install (the `install-knope` action, below), `knope-pin.test.sh` (no
    workflow carries a knope pin of its own) and `knope-closes-compat.test.sh` (the release tool must
    tolerate a fragment's `closes` key);
  - `cargo build --release --locked`;
  - the planted-violation cases of `release-guard.test.sh` and `release-invariants.test.sh`, then
    `release-invariants.sh` on the ref being checked;
  - `make rule-table-check` (the `SPEC-2` rule table is current);
  - the tests through `.github/scripts/test-no-leak.sh` (a fresh `TMPDIR` that must be empty of `urzua-*`
    entries afterwards), after the guard's own planted-violation cases;
  - a fetch of the pull-request head refs into a private namespace, which `identity.open-pr-collision`
    reads (`ADR-93`; `check` never fetches);
  - the **urzua gate** action over the binary just built: `urzua check`, and on a pull request
    `urzua check --base origin/<base>` with the author and approvers as `--actors`, reported as
    annotations and one updating comment. `base-ref` is the base branch on a pull request and the branch
    a queued commit will land on in a merge queue (`merge_group`), so the diff rules run for the commit
    about to merge; it is empty on a push and on a tag, where only the whole-tree check runs. The
    workflow's `comment` input (default `"true"`) lets a caller keep the gate from commenting;
  - `urzua harness status` (the installed hooks and commands have not drifted).
- **`changeset`** (`changeset required`): requires a `.changeset/*.md` fragment added in the diff, or
  the `no-changeset` label (`ADR-29`) — never a silent skip. Runs on `pull_request` only, and **only asks
  at all when the PR touches a path declared in `.github/release-paths`** (`rust/`, `README.md`,
  `LICENSE` — `ADR-49`). A PR touching none of them cannot affect what an installer receives, so it is
  not asked for a judgement the repository's structure already determines; it passes with a log line
  saying so. That set is deliberately not `publish-release.yml`'s packaging list: this names paths whose
  *change* can affect the archive, that one names files copied *into* it. When the gate does apply, the
  label is read from **the PR's current state, not from `github.event`** (`BUG-34`): a payload is a
  snapshot taken when the event fired, and a re-run replays the same stale snapshot. The job therefore
  always starts and reports why it skipped, rather than being skipped by a job-level `if:`. The
  `labeled`/`unlabeled` triggers (`BUG-15`) start a fresh run promptly when a human labels a PR after
  opening it.
- **`release-invariants`**: the planted-violation cases and the invariants themselves, on a full-history
  checkout with tags (`BUG-93`). A manifest version ahead of the newest tag while fragments are pending
  fails, and so does a non-`major` fragment that opens a line with `Breaking:`.
- **`gate-action`**: the gate action's own tests (`.github/actions/urzua-gate/test.sh`): recorded-fixture
  rendering, update-in-place commenting, the history precondition and the installer's refusals.
- **`gate-released`** (`gate (pinned release)`): the pinned release, installed through the same unit an
  adopter uses (checksum and attestation verified), run over this repository's corpus as of that release
  tag. Its version, sha256 and ref are bumped together by hand when a release is cut.

Concurrency is workflow-level, so a new commit cancels the whole superseded run, including on a PR title
or body edit (`edited`).

**`make ci`** runs the checks of this section that can run on a laptop, including `check --base
origin/main` (after fetching the pull-request refs) and the knope pin test. It does not run the changeset
label gate, the pinned-release gate (a download with an attestation check) or the `--actors` file, which
need the GitHub API; CI runs all three, and the target says so rather than claiming to be everything CI
runs.

## `prepare-release.yml` / `publish-release.yml` — the two-step release (`ADR-45`)

- **`prepare-release.yml`** runs on every push to `main`, guarded against re-triggering on its own
  release-prep commit message. It force-pushes a `release` branch built from `main`, running knope's
  `prepare-release` workflow (`PrepareRelease` → `cargo check` → `harness update` → commit → push →
  `CreatePullRequest` → label), which compiles every accumulated `.changeset/*.md` fragment into a version
  bump and `CHANGELOG.md` section. **Fragments are the only trigger** — `ignore_conventional_commits`
  is set, because a commit prefix infers "releasable" from a message with no knowledge of which tree
  changed, and only `rust/` produces the released artifact (`ADR-48`, `BUG-35`). The job
  installs the pinned toolchain and a cargo cache first, because that `cargo check` needs both
  (`BUG-27`). **No `continue-on-error`**: swallowing every failure hid a six-day release outage once.
  Instead the step tolerates **one documented error by name** — knope's `releases::no_release`, which
  a merge carrying no changeset legitimately produces (`BUG-33`) — emits a `::notice::` saying so,
  and fails on anything else with its original exit code, except that `release-guard.sh` turns a
  `no_release` with fragments still present into a failure (`BUG-48`). Tolerating the error by name rather
  than counting fragments up front also survives `ADR-48`: the count and the trigger now agree, but the
  step does not depend on their agreeing.
- **The release PR is opened by a GitHub App, not `GITHUB_TOKEN`** (`ADR-95`, which builds part of `RFC-30`;
  the ADR is `Proposed` and `RFC-30` stays `Draft`). GitHub starts no workflow run from a `GITHUB_TOKEN`
  event, so a PR opened that way never gets its `ci` run, and `ci` could not be a required check on `main`.
  The job mints an installation token with `actions/create-github-app-token` from the App's client id
  (`RELEASE_APP_CLIENT_ID`) and private key (`RELEASE_APP_PRIVATE_KEY`), both repository secrets, requesting
  only `contents: write` and `pull-requests: write`, and fails with an explicit error when either secret is
  unset (there is deliberately **no fallback** to `GITHUB_TOKEN`). It uses the token for the checkout, the
  `release` branch push, `CreatePullRequest` and the label. The job holds no write grant on its own
  `GITHUB_TOKEN`, because nothing uses one, and the token is narrower than the App's installation because the
  job runs repository code (`cargo check`, `harness update`) with it in `.git/config`. Once the App and secrets
  are set, the release PR is an ordinary pull request whose `ci` run executes without approval. The App, the
  secrets and the ruleset on `main` are repository settings, not files in this repository, and the steps to
  set them are in `docs/reference/release-app-setup.md`. **The ruleset is required and not yet applied:**
  nothing yet makes `ci` a required check on `main`. It will target `main` only (not `release`, not `v*`
  tags), require a pull request with 0 approvals and the `ci` check, block force pushes and deletions, and have
  an empty bypass list. Until the secrets are set, `prepare release` fails on every push to `main`.
- The `cargo check` step exists because `PrepareRelease` only regex-bumps `Cargo.toml`'s
  version line — it has no knowledge of `Cargo.lock`, which independently records each workspace
  member's resolved version and goes stale the instant `Cargo.toml` changes without it (`BUG-14`,
  found on this mechanism's first real run: every `--locked` build/test/clippy invocation in this
  repo failed against the resulting PR). Not `--offline`: that needs an already-populated registry
  the release job doesn't have on a cold cache, and fails resolving external deps rather than doing
  the local-only refresh it was meant to (`BUG-27`). Not `--locked` either — the lockfile is stale by
  construction at that point, which is the whole reason the step exists. The committed `Cargo.lock`
  still pins every external version, so the check rewrites workspace members and nothing else. The
  `harness update` step re-pins `.urzua/harness.lock` to the bumped version in the same commit, so
  `urzua harness status` does not go red on the release PR.
- The resulting PR **is** the reviewable release artifact — the actual version bump and compiled
  changelog, visible as a real diff, not trusted sight-unseen inside a single dispatched job. Covered
  by the `no-changeset` label: it consumes fragments, it doesn't add one.
- **`publish-release.yml`** runs when that specific PR (head branch `release`) merges into `main`,
  and owns the entire publish — tag, GitHub Release, binaries, upload. Detailed below.
- **One knope pin.** `prepare-release.yml`, `publish-release.yml` and `checks.yml` install knope through the
  `.github/actions/install-knope` composite action, the only place the version, URL and checksum are
  written. `knope-pin.test.sh` fails when any workflow carries a download of its own or stops using the
  action, and `knope-closes-compat.test.sh` reads the pin from the action, so a knope upgrade is one file
  and re-runs the compat test against the new version.

## `publish-release.yml` — the whole publish, in one workflow

Four jobs in sequence. **One workflow rather than two deliberately** (`BUG-28`): the tag is pushed
with `GITHUB_TOKEN`, and GitHub never triggers a workflow from a `GITHUB_TOKEN` event, so a separate
tag-triggered build workflow silently never runs — publishing a release with no binaries and no
error. `release.yml` previously held the last three jobs and is deleted.

- **`verify`**: calls `checks.yml` (`ADR-47`) against the merge commit (`merge_commit_sha`: for a
  squash merge, the commit created on `main`, which is the commit knope tags below), so the tree being
  released is the tree that was just checked. It runs only for a merged PR whose head branch is `release`
  in this repository. It does not ask whether a `ci` run happened elsewhere: a cross-workflow query
  could only report whether someone approved a held run, and the merge commit's own `ci` run races this
  workflow. `release` `needs` it, so a red `verify` publishes nothing. The gate's pull-request comment is
  switched off here (`comment: "false"`): the event is the merged release PR, whose comment would be
  rewritten for a tree nobody is reviewing, and the job still holds `pull-requests: write` because the
  called job asks for it.
- **`release`**: runs knope's `release` workflow (just the `Release` step) — tags the now-current
  version and creates the GitHub Release, with the compiled changelog as its notes since `[github]`
  config is present. Outputs the resolved tag, read from the same anchored `rust/Cargo.toml` line
  `knope.toml` itself bumps.
- **`build`**: cross-compiles for `aarch64-apple-darwin`, `x86_64-apple-darwin` (both from the same
  `macos-14` arm64 runner — a dedicated Intel runner queues indefinitely as GitHub winds down that
  capacity), and `x86_64-unknown-linux-gnu`. Checks out the **tag**, not a branch head, so a push
  landing on `main` mid-publish cannot produce binaries that disagree with their release.
- **`upload`**: `gh release upload --clobber` against the release `knope release` already created — never
  `gh release create`, which would collide with an existing release object. Then **asserts the
  result**: reads the release back and fails if fewer than three archives are attached. A release
  object existing is not the same as a release being installable.
- GitHub Release binaries only (`ADR-13`) — not published to crates.io; `Cargo.lock` is committed for
  this reason (a binary, not a library other crates depend on).

**No manual step remains** beyond merging the release PR, which is `ADR-45`'s deliberate act. The
release PR's own `ci` run executes, because a GitHub App opened it (`ADR-95`), and the merge into `main`
is gated by the ruleset's required `ci`. `verify` still re-runs the checks against the merge commit, so
nothing relies on the pull request's run having happened.

**If `verify` fails after the release PR has merged**, `main` carries the bumped version (for example
0.4.0) with no tag. `release-invariants.sh` then fails every later pull request that has pending fragments,
because the manifest leads the newest tag (`BUG-92`), until the publish run succeeds. Fix the cause, then
re-run the failed `publish release` workflow run; do not tag by hand.

The changeset gate once failed on the release PR because a re-run replayed the **stale `opened` payload**,
taken before knope's own labelling step (`BUG-34`). The gate now reads the PR's current labels at run time
rather than from the payload, which makes staleness unrepresentable — including on a re-run, which
replays the original payload.

## `knope.toml` / `.changeset/*.md` — the fragment format (`ADR-29`)

- `[package].versioned_files` is an explicit, anchored-regex list (`(?m)^version = "..."$` for
  `rust/Cargo.toml`) — an unanchored pattern would also match every `{ version = "...", ... }`
  dependency pin in `[workspace.dependencies]` and `rust-version` itself, silently corrupting them on
  the next bump (verified against a live dry run before trusting it).
- `[github]` config (owner/repo) is required for `CreatePullRequest` and `Release` to reach the
  GitHub API — added by `ADR-45`, absent under the original single-dispatched-job design.
  `[[workflows]]` declares two named workflows, `prepare-release` and `release`, invoked as `knope
  prepare-release` / `knope release` respectively.
- A fragment (`.changeset/*.md`) is the sole release trigger (`ADR-48`) and is real changesets
  format: `default: major | minor | patch`
  frontmatter plus a Markdown body, one per user-visible PR. On a pre-`1.0.0` package, a `major`
  fragment bumps the minor version position per standard semver pre-1.0 convention (knope's default),
  never jumps straight to `1.0.0` on its own.
- A `README.md` with no changesets frontmatter inside `.changeset/` fails the release step outright
  if knope tries to parse it as a fragment — caught in a dry run before it shipped (`ADR-29`).

## What's deliberately not built

- **A hosted knope GitHub App** (`bot.releases`) — knope offers this as an alternative to running the
  CLI directly in a workflow; not adopted here, since the existing pinned-checksum CLI install
  already works. The App that does exist is this repository's own, minted per run for the release PR
  (`ADR-95`), not a hosted knope service.
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
- ADR-47 — `publish-release.yml` verifies the tree it ships through the shared `checks.yml`.
- ADR-95 — the release PR is opened by a GitHub App token and `main` requires `ci` (`Proposed`).
- RFC-30 — the proposal `ADR-95` builds part of; still `Draft`.
- ADR-93 — the pull-request refs fetch and `identity.open-pr-collision`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial spec. **Why:** the CI/CD pipeline had real decisions spread across four ADRs and three workflow files with no single buildable reference — found live while building `ADR-45`'s two-step release flow, the same "coherent feature area, decided editorially" gap `ADR-41` already named for record types, here applied to infrastructure for the first time. | **structural** |
> | 2026-09-10 | Documented two fixes found on this mechanism's first real run, same-day as `ADR-45` merged: `BUG-14` (`prepare-release`'s missing `Cargo.lock` regeneration) and `BUG-15` (`ci.yml`'s changeset gate missing `labeled`/`unlabeled` trigger types). **Why:** this spec's own standing purpose is to stay the accurate, buildable reference — leaving it describing the pre-fix mechanism the same day it changed would be the exact doc-drift gap this project's own `AGENTS.md` now explicitly guards against. | **substantive** |
> | 2026-09-16 | Corrected three claims, all disproved by the `v0.2.1` run: the `no-changeset` no-op (`BUG-33`), `verify-ci`'s "it gated the merge" justification (`main` is unprotected), and the manual gate being one step rather than two (`BUG-34`). **Why:** this spec's standing purpose is to be the accurate, buildable reference, and each of these read as settled while being false -- the kind of claim that survives precisely because it is written down. | **substantive** |
> | 2026-09-16 | Rewrote the release section: `publish-release.yml` now owns the whole publish (tag, release, binaries, upload) and `release.yml` is deleted. Documented the previously-undocumented manual approval the release PR's own `ci` run requires. **Why:** `BUG-28` -- the tag is bot-pushed, GitHub never triggers a workflow from a `GITHUB_TOKEN` event, so the separate tag-triggered build workflow silently never ran and `v0.2.0` published with zero binaries. This spec described a coupling that did not exist. | **substantive** |
> | 2026-11-02 | Bumped to `0.4`. `Implements` no longer lists `ADR-29`. **Why:** `ADR-29` is Superseded by `ADR-45`, which was already listed and carries forward `knope`, the fragment format and the versioned-files config unchanged; `pointer.target-status` flagged the stale target. The body still cites `ADR-29` in prose for the fragment format and the changeset gate, which is lineage, not a claim to implement it. | **structural** |
> | 2026-11-04 | Bumped to `0.5`. The `rust` job also runs `cargo machete` (unused dependencies) and `cargo deny check` (advisories, bans, licenses, sources; `rust/deny.toml`), both pinned prebuilt binaries verified by checksum; `make deny` and `make ci` run them locally. **Why:** `MILE-176` -- `yaml_serde` is a young single-supplier crate, and nothing failed on an unused dependency. | **substantive** |
> | 2026-11-05 | Bumped to `0.6`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:.github/workflows/ci.yml, code:.github/workflows/publish-release.yml, test:.github/scripts/release-guard.test.sh, test:.github/scripts/release-invariants.test.sh, test:.github/actions/urzua-gate/test.sh` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.7`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
> | 2026-11-05 | Bumped to `0.8`. The `rust` job runs the tests through `.github/scripts/test-no-leak.sh`, under a fresh `TMPDIR`, and fails when a run leaves any `urzua-*` entry behind; `make ci` does the same (`make test-no-leak`). **Why:** `BUG-193` -- fixture directories accumulated on every run until the disk filled, and nothing measured it. | **substantive** |
> | 2026-11-07 | Bumped to `0.9`. Rewrote the pipeline sections to match the workflows: the gate is `checks.yml`, called by `ci.yml`'s `rust` job and by `publish-release.yml`'s `verify` job (there is no `verify-ci` job; `ADR-47` deleted it); `ci.yml` also has `release-invariants`, `gate-action`, `gate-released` and the aggregate `ci` job, the one required check; the release PR is opened by a GitHub App token, so its `ci` run executes (the claim that it never runs, and that `RFC-30` was the only mitigation, is removed); added the recovery when `verify` fails after the release PR merged. `Realized-by` gains `checks.yml` and `prepare-release.yml`. **Why:** the maintainer decided on 2026-11-05 to protect `main` with `ci` required and open the release PR with an App token (`ADR-95`), and the spec still described a deleted job and a run that never executes, which would mislead anyone debugging a red release PR or setting the ruleset. | **substantive** |
> | 2026-11-07 | Bumped to `0.10`. Documents the follow-up hardening of the release workflows: the App token is minted from the client id and requests only `contents` and `pull-requests` write, with no write grant on the job's own `GITHUB_TOKEN`; knope is installed through one `install-knope` action that `knope-pin.test.sh` guards, instead of a pin copied into three workflows and checked in one; the gate takes the queued commit's base branch in a merge queue, so the diff rules run there; `checks.yml` takes a `comment` input that `publish-release.yml` sets to `"false"`; `upload` passes `--clobber` so a re-run after a partial upload completes; `make ci` runs `check --base` and the knope pin test and names the three checks it leaves to CI. `Realized-by` gains the action and the pin test. **Why:** an independent review found the App token broader than its job, the knope pin copied into three workflows and checked in one, and a required check that ran no diff rules in a merge queue; `make ci` claimed to be everything CI runs. | **substantive** |
> | 2026-11-07 | Bumped to `0.11`. The release-App paragraph no longer reads as though the ruleset on `main` is applied: it says the ruleset is required and not yet applied, states its scope (`main` only, 0 approvals, `ci` required, empty bypass list), says `prepare release` is red on every push until the secrets are set, and points to `docs/reference/release-app-setup.md`. **Why:** an independent review of `ADR-95` could not see a ruleset on the repository, and the spec said the merge was gated by it. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 10 locators pinned, 10 file-level, 0 scoped. Evidence: .github/workflows/ci.yml@sha256:3d22f5501990, .github/workflows/checks.yml@sha256:4afa5fe0db59, .github/workflows/prepare-release.yml@sha256:0e4d28280d1a, .github/workflows/publish-release.yml@sha256:530bdeeececf, .github/actions/install-knope/action.yml@sha256:ace7ea7ca992, .github/scripts/knope-pin.test.sh@sha256:2c13f4fc3ca3, .github/scripts/release-guard.test.sh@sha256:5fca6fe029d9, .github/scripts/release-invariants.test.sh@sha256:8fc2b5758338, .github/scripts/test-no-leak.test.sh@sha256:36c44d3cd4c5, .github/actions/urzua-gate/test.sh@sha256:2d9fb3612a1d Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml and the own-config test still declare the adr, rfc, spec and condition types that SPEC-16, SPEC-17, SPEC-18 and SPEC-23 describe, with spec_shipped_integration.rs, rules/roadmap.rs, condition_type_integration.rs and work_types_integration.rs behind them, check.rs, rules.rs, check_integration.rs and fail_on_integration.rs still implement urzua check (SPEC-2), the workflows, the install-knope action and the release and gate test scripts still make up the pipeline SPEC-20 describes, the Makefile and checks.yml still dispatch the repository layout of SPEC-21, the harness modules, the bundled hook script and the harness integration tests still implement the request, verdict and adapter layout of SPEC-24, config.rs and the own-config and field-levels tests still implement SPEC-3, and acceptance.rs and acceptance_property.rs still implement SPEC-4. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.12`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
