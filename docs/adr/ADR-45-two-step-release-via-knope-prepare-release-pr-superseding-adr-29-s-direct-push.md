---
Stable-Id: 01M25Z27VBE63ABNE2QRSQSXGR
Status: Accepted
Embodiment: Verified
Realized-by: code:knope.toml, code:.github/workflows/prepare-release.yml, code:.github/workflows/publish-release.yml, code:.github/workflows/release.yml
Date: 2026-09-10
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: ADR-29
Derives-from: —
---
# 45 — Two-step release via knope prepare-release PR, superseding ADR-29's direct push

## Context

ADR-29 decided releases are cut by a maintainer running `prepare-release.yml` (`workflow_dispatch`),
which runs `knope release` end to end -- bump the version, compile `CHANGELOG.md` from every
accumulated `.changeset/*.md` fragment, commit, tag, and push, all in one step, directly to `main`.
That was never the intended shape: the actual want was a two-step process where the compiled
version bump and changelog are visible as a real diff *before* they land, not trusted sight-unseen
inside a single dispatched job. ADR-29's own Decision text is explicit about the opposite --
*"a direct, reviewed push, never an unattended action on every merge to main"* -- which describes
today's mechanism accurately but not the actual intent behind writing it that way.

Concretely surfaced by this repo's own accumulated state: 19 changesets currently sit in
`.changeset/`, unreleased, with no way to see what version bump or changelog they'd actually compile
to without running the release for real. One of them (`stdout-always-json.md`) is tagged
`default: major` -- a real, correct tag for the change it describes (the `--format` flag's removal),
but with no preview step, there's no way to sanity-check what version knope would actually compute
before committing to it.

`knope` itself already ships this exact two-step shape as a documented, first-class recipe (its
`PrepareRelease` step separately from its `Release` step, plus a `CreatePullRequest` step) -- nothing
about ADR-29's tool choice needs revisiting, only how its steps are sequenced across workflows.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Keep ADR-29 as-is (single dispatched `knope release`) | Already built, zero new surface | The compiled version/changelog is never visible before it lands on `main` -- exactly the review gap this decision exists to close |
| Two knope-driven workflows: `prepare-release` opens/updates a PR, `publish-release` runs on that PR's merge | Matches knope's own documented recipe; the PR *is* the reviewable artifact, matching what "reviewed" was always meant to mean; no new dependency (still the same pinned, checksummed knope binary already in use) | A real behavior change to `release.yml`'s own release-creation step (below); requires enabling org-level "Actions can create pull requests," a one-time repo/org setting |
| A JS-based bot (e.g. an off-the-shelf changesets GitHub Action) | Well-trodden, less workflow YAML to hand-write | ADR-28 already rejected exactly this category of tool for this repo (a Node dependency for changelog tooling) -- nothing about that reasoning has changed |

## Decision

In the context of a release mechanism that was built as a single direct push when the actual intent
was always a reviewable two-step process, facing 19 unreleased changesets with no preview of what
they'd compile to, we decided: **replace the single dispatched `knope release` with two workflows,
using knope's own documented `PrepareRelease`/`CreatePullRequest`/`Release` steps** --

- `prepare-release.yml` runs on every push to `main` (guarded against re-triggering on its own
  release-prep commit) and opens or updates a PR from a `release` branch into `main`, containing the
  compiled version bump and changelog. `continue-on-error: true` on the knope invocation itself,
  matching knope's own documented recipe: when there's nothing to release, `PrepareRelease` (or the
  commit step right after it) fails fast and the rest of the workflow never runs, without turning a
  routine docs-only merge into a red check.
- `publish-release.yml` runs when that specific PR merges into `main`, and runs `knope release`
  (just the `Release` step) -- tags the now-current version and creates the GitHub Release, which
  fires the existing tag-triggered `release.yml` unchanged, except for one consequence below.
- `no-changeset` covers this new release PR the same way it already covers any other PR with
  nothing new to tell an installer -- the release PR consumes fragments, it doesn't add one.

`knope`, `changesets`-format fragments, and the underlying versioned-files/changelog config are all
unchanged from ADR-29 -- only the sequencing across workflows changes.

## Reversibility

Cheap to revert: delete the two new workflow files, restore `prepare-release.yml`'s single
`workflow_dispatch` job calling `knope release` directly, and the `release` branch/PR simply stops
being created. No data format changes -- `.changeset/*.md` fragments and `CHANGELOG.md` are byte-
identical in shape either way. The org-level Actions setting can stay enabled with no consequence for
any other workflow in this repo.

## Consequences

- Every push to `main` that leaves something unreleased keeps a "release" PR open and current --
  the compiled version/changelog is always visible as a real diff, closing the review gap ADR-29's
  own text described without actually building.
- `release.yml`'s `publish-release` job changes from `gh release create ... --generate-notes` to
  `gh release upload` against the release `knope release` already created (with real, changelog-
  sourced notes) -- a real behavior change, not just additive, since `knope`'s `Release` step
  creates the GitHub Release itself once `[github]` config exists (needed regardless, for
  `CreatePullRequest`). Auto-generated PR-list notes are replaced by the actual changelog text.
- Requires "Allow GitHub Actions to create and approve pull requests" enabled at the **organization**
  level (`urzuahq`), not just the repo -- the repo-level API call alone returned a 409 until this was
  done; a one-time setting, not a per-release action.
- The `release` branch is long-lived and force-pushed on every `prepare-release` run, the same way
  the recipe itself does it -- never a source of real work, always disposable.

## References

- ADR-29 -- the decision this supersedes; its choice of `knope` over a JS-based tool stands
  unchanged, only the single-dispatched-job sequencing is replaced.
- ADR-28 -- the original rejection of a Node-based changelog tool, the reasoning this decision does
  not revisit.
- `knope.toml`, `.github/workflows/prepare-release.yml`, `.github/workflows/publish-release.yml`,
  `.github/workflows/release.yml` -- this decision's realization.
- SPEC-20 -- the CI/CD pipeline spec this decision's mechanism is documented under.
