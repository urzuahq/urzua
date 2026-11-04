---
Stable-Id: 01M25Z27VBE63ABNE2QRSQSXGR
Status: Accepted
Embodiment: Implemented
Realized-by: code:knope.toml@sha256:8557a6d8ecaa3736fb1ebc6ad20b976967de3289bd0e1be6ec5ac8ffe529f9d9, code:.github/workflows/prepare-release.yml@sha256:50c0f37fa931de7590fb4970a294785df44f88375e3075883115fc4a01515c19, code:.github/workflows/publish-release.yml@sha256:f8fb928d07b3111e943f644be05b48656b7e796bdd8d0b4fb86237400b173591
Date: 2026-09-10
Accepted-on: 2026-09-10
Author: beauwilliams
Deciders: beauwilliams
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

## Amendment (2026-11-04): the release gate and the guard on an empty release are those of `ADR-47` and `BUG-48`

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

Several statements in the Decision and in the two earlier amendments no longer describe the
workflows. The two-step flow (a prepare workflow opening the release PR, a publish workflow running
when it merges) stands.

- **`continue-on-error: true` is gone.** The Decision has `prepare-release.yml` carry it on the
  knope invocation. The workflow now tolerates only knope's `no_release` outcome, through
  `.github/scripts/release-guard.sh` (`BUG-48`), and fails on anything else.
- **`verify-ci` is replaced.** The first amendment says `verify-ci` checks the release PR's head sha,
  and the second that it asks whether a human approved the run. `publish-release.yml` now runs
  `.github/workflows/checks.yml` itself on the PR's `merge_commit_sha`, the commit that is tagged
  (`ADR-47`). Whether the PR's own `ci` ran or was approved no longer decides the publish.
- **"The one manual step per release" no longer holds.** The `action_required` approval described in
  the second amendment is not a publish gate, because the checks run inside the workflow the merge
  triggers.
- **Changesets alone trigger a release.** `knope.toml` sets `ignore_conventional_commits = true`
  (`ADR-48`), so a conventional commit no longer triggers a release; the Decision's "unchanged from
  `ADR-29`" for the changeset fragments stands.

## References

- ADR-29 -- the decision this supersedes; its choice of `knope` over a JS-based tool stands
  unchanged, only the single-dispatched-job sequencing is replaced.
- ADR-28 -- the original rejection of a Node-based changelog tool, the reasoning this decision does
  not revisit.
- `knope.toml`, `.github/workflows/prepare-release.yml`, `.github/workflows/publish-release.yml`
  -- this decision's realization.
- SPEC-20 -- the CI/CD pipeline spec this decision's mechanism is documented under.
- BUG-27, BUG-28 -- the two defects the first real end-to-end run of this flow exposed; see the
  amendment below.

## Amendment (2026-09-16): one workflow owns the publish; `release.yml` is gone

This decision's Consequences said the release tag `knope release` pushes "fires the existing
tag-triggered `release.yml` unchanged." **That is false, and was never true.** GitHub does not
trigger workflows from events created with `GITHUB_TOKEN` — a deliberate recursion guard. Under
ADR-29 a maintainer pushed the tag by hand, so `release.yml` fired; under this decision a bot pushes
it, so `release.yml` has not run since 2026-09-05. `v0.2.0` published with a correct changelog and
zero binaries before anyone noticed (BUG-28).

Decided: `publish-release.yml` owns the entire publish — `verify-ci`, `release`, `build`, `upload` —
and `release.yml` is deleted. The cross-workflow trigger this decision depended on does not exist, so
the dependency is removed rather than worked around. Two consequences of that:

- **`verify-ci` moves into `publish-release.yml` and checks the release PR's head sha**, not the
  merge commit — the head has definitively concluded (it gated the merge), while `ci` on the merge
  commit races this workflow. The release-gating property this decision claimed is preserved, on a
  commit where it can actually be evaluated.
- **The upload job asserts the release carries its three archives** and fails otherwise. A release
  object existing is not the same as a release being installable, and this flow had no check that
  could tell those apart.

Two further corrections to this decision's original text, both found in the same first real run:

- The `cargo check --offline` step this decision's `knope.toml` introduced could never succeed in
  the job it ran in — no toolchain, no registry (BUG-27). `--offline` is dropped and
  `prepare-release.yml` now installs the toolchain.
- **The release PR's own `ci` run requires manual approval.** It is triggered by a bot-pushed branch,
  so GitHub holds it in `action_required` until a human approves. This decision documented no such
  step; it has been a silent manual gate in the flow since it shipped, and it remains one. Removing
  it needs either a PAT (re-opening the recursion risk this decision avoided) or the checks to move
  inside `publish-release.yml` too — undecided here, named so the next person doesn't rediscover it
  as a surprise.

## Amendment (2026-09-16, second): two claims the first amendment got wrong

Shipping `v0.2.1` exercised this flow end to end for the first time without a defect consuming the
run. Two statements in the amendment above are false.

**"the head has definitively concluded (it gated the merge)".** It gated nothing. `main` has **no
branch protection**, so no check is required and the release PR is mergeable with none at all —
verified directly on `#39`, which reported `MERGEABLE` while its `ci` had never run. The reason to
check the head sha rather than the merge commit still holds (the merge commit's `ci` races this
workflow), but the stated justification does not.

The consequence is sharper than the wording: `verify-ci` reads `missing` when `ci` has not run, and
`ci` cannot run on a `GITHUB_TOKEN`-created PR without a human approving it. So `verify-ci` is not
"did the tree pass" — it is "did a human approve the run." Merging the release PR before that
approval publishes nothing and leaves `main` carrying a version bump with no tag.

**The manual gate was two steps, not one.** This amendment recorded the `action_required` approval.
Approving it replayed the *stale* `opened` payload, which predates knope's own labelling step, so the
changeset gate ran when it should have skipped and failed — asking for a label that was already
present. `BUG-34` fixed that half by reading the PR's live labels instead of the payload. The
approval itself remains, and is the one manual step per release.

Both trace to one cause this decision now depends on three times over: `GITHUB_TOKEN` events do not
trigger workflows. `BUG-28` was fixed by deleting the dependency; the remaining two are GitHub's own
mechanisms and cannot be removed the same way. `RFC-30` proposes acting as a GitHub App instead.

This record's own `Embodiment` is corrected here too, from `Verified` to `Implemented`.
`embodiment.consistency` had been flagging it: every `Realized-by` locator is a `code:` one, and
nothing tests this flow, so `Verified` was a tier the evidence never supported. A decision record
overstating its own embodiment while its prose overstated its own guarantees is the same defect
twice, and only one of them was machine-checkable.

Also recorded: `prepare-release` now fails on every `no-changeset` merge, because knope treats
nothing-to-release as an error (`BUG-33`). That is the cost of `BUG-27` removing
`continue-on-error`, and the right trade — but a workflow that goes red on routine merges is a gate
nobody reads.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Implemented`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:knope.toml, code:.github/workflows/prepare-release.yml, code:.github/workflows/publish-release.yml` -- stated `Implemented` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:knope.toml, code:.github/workflows/prepare-release.yml, code:.github/workflows/publish-release.yml` -- stated `Drift detected` disagreed with the computed value. | **structural** |
> | 2026-11-04 | `Embodiment` restored to `Drift detected`. **Why:** the previous row recomputed `Implemented` in a shallow checkout, where git history is missing and drift cannot be seen. In a full clone `knope.toml` (2026-09-16) and `.github/workflows/prepare-release.yml` (2026-09-19) are newer than this record's `Realized-by` line, so the evidence did change after it was last verified, and `embodiment.consistency` reports the mismatch. Found when `MILE-145` made warnings fail the build. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 3b0fe6682. | **structural** |
> | 2026-11-04 | Amendment added: the empty-release guard (`BUG-48`), `ADR-47`'s checks on the merge commit in place of `verify-ci`, and `ADR-48`'s changesets-only trigger replace what the Decision and its two amendments describe. **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Pinned `knope.toml`, `.github/workflows/prepare-release.yml`, `.github/workflows/publish-release.yml` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-pinned with the amendment above: the two-step flow holds and the changes since the last verification are recorded. | **structural** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:knope.toml@sha256:8557a6d8ecaa3736fb1ebc6ad20b976967de3289bd0e1be6ec5ac8ffe529f9d9, code:.github/workflows/prepare-release.yml@sha256:50c0f37fa931de7590fb4970a294785df44f88375e3075883115fc4a01515c19, code:.github/workflows/publish-release.yml@sha256:f8fb928d07b3111e943f644be05b48656b7e796bdd8d0b4fb86237400b173591` -- stated `Drift detected` disagreed with the computed value `Implemented`. | **structural** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
