---
Status: Done
Stable-Id: 01M1YKX8C584N9ESVE5181FJJE
Phase: '1'
Track: release-process
Implements: ADR-45
Blocked-on: —
Realized-by: code:.github/workflows/prepare-release.yml@sha256:0e4d28280d1a77819841035b7ed7da5ee570cf96e80875d2f1e8cc83d7596b8d, code:knope.toml@sha256:a0daf73a621a5ddd0e1e9735e6a324179ec9754573ebc4146e7631f25b1d9bc4, code:.github/scripts/release-guard.sh@sha256:d4ea7d90a19766e0b90b00ecfa5b3eb3b18bd01d27de44fb630320c645b90078
---
# 39 — Auto-open a release PR alongside manual workflow_dispatch

## What

_Original plan, written while `ADR-29`'s manual `workflow_dispatch` release was in force. `ADR-45` has since replaced it with a reviewable release PR; this milestone now records that as its `Implements`._

`prepare-release.yml` currently only supports a manual `workflow_dispatch` that pushes the version
bump and compiled `CHANGELOG.md` directly to `main` (ADR-29's decided mechanism). Add a second path:
on push to `main` when unreleased changesets exist, automatically open (or update) a release PR
carrying that same staged version bump and compiled changelog, so a maintainer reviews and merges a
normal PR instead of having to remember to trigger the dispatch. Keep `workflow_dispatch` working
alongside it. Requires amending ADR-29's decision (currently states "a direct, reviewed push, never
an unattended action on every merge to main") -- opening a PR is not itself cutting a release
(merging still is), but the mechanism described there needs updating, transparently, not silently.

## Why

13 changesets have accumulated since v0.1.0 with no release cut (MILE-40) -- partly because cutting
one requires a maintainer to remember `workflow_dispatch` exists and go trigger it by hand. A PR
that opens itself is a standing, visible reminder instead of a step nobody's watching for.

## Done means

- `prepare-release.yml` triggers on push to `main`. — evidence: code:.github/workflows/prepare-release.yml@sha256:0e4d28280d1a77819841035b7ed7da5ee570cf96e80875d2f1e8cc83d7596b8d
- When unreleased changesets exist, `prepare-release.yml` opens or updates a release pull request carrying the staged version bump and compiled `CHANGELOG.md`, so a maintainer reviews and merges a normal PR. — evidence: code:.github/workflows/prepare-release.yml@sha256:0e4d28280d1a77819841035b7ed7da5ee570cf96e80875d2f1e8cc83d7596b8d, code:knope.toml@sha256:a0daf73a621a5ddd0e1e9735e6a324179ec9754573ebc4146e7631f25b1d9bc4, code:.github/scripts/release-guard.sh@sha256:d4ea7d90a19766e0b90b00ecfa5b3eb3b18bd01d27de44fb630320c645b90078
- `prepare-release.yml` has no trigger that pushes a release straight to `main` (`ADR-45` replaced the `workflow_dispatch` path this milestone first planned to keep). — evidence: code:.github/workflows/prepare-release.yml@sha256:0e4d28280d1a77819841035b7ed7da5ee570cf96e80875d2f1e8cc83d7596b8d

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | `Blocked` -> `Done`. **Why:** `ADR-45` built this -- `prepare-release.yml` runs on every push to `main` and opens or updates the release PR, which is how `#24` (v0.2.0) and `#39` (v0.2.1) were cut. The `Blocked-on: MILE-38` sequencing was deliberate at the time but overtaken: `ADR-45` superseded `ADR-29`'s mechanism outright rather than amending it, so the amended-citation test case this was waiting to provide never arose. `MILE-38` is unaffected and still `Planned`. | **substantive** |
> | 2026-11-02 | `Implements` repointed from `ADR-29` to `ADR-45`. **Why:** `ADR-29` is Superseded and `ADR-45` is the decision now in force for this work; `pointer.target-status` flagged the stale target. | **structural** |
> | 2026-11-02 | Labelled the body as the original `ADR-29`-era plan. **Why:** review found the body still described a release flow `ADR-45` replaced. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Superseded in part by `ADR-45`; `Status` stays Done. **Why:** the closure audit of #234 found that the instruction to keep `workflow_dispatch` working no longer holds: `prepare-release.yml` has only a `push` trigger on `main`, the release PR being the mechanism `ADR-45` decided. What this milestone delivered, the release PR opened or updated automatically on push to `main`, is unchanged. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: .github/workflows/prepare-release.yml@sha256:0e4d28280d1a, knope.toml@sha256:08a547f0466a, .github/scripts/release-guard.sh@sha256:d4ea7d90a197 Reason: Pinned the files as they are now after checking each against the record it evidences: schema_version.rs, commands/migrate.rs, table.rs and config.yaml still carry the schema version window and the cross-record rules (MILE-34), AGENTS.md still instructs agents to track work in the record types, verify against real runs, never silently rewrite an Accepted decision and not build speculative capability (MILE-36), prepare-release.yml still triggers only on push to main and opens a release pull request with knope.toml and release-guard.sh (MILE-39), section.rs still implements required sections and the y-statement shape (MILE-4, MILE-5), CHANGELOG.md, rust/Cargo.toml and the release workflows still show releases past v0.1.0 consuming changesets (MILE-40), report.rs, action.rs, table.rs and generate-rule-table.py still give every finding a suggested action (MILE-44), report.rs, config.rs and runner.rs still carry the info level (MILE-46), document_model.rs still reads a second corpus's declared model (MILE-51), and the Phase A rule modules, config.yaml and checks.yml still hold the header, pointer, field, filename and supersession rules (MILE-56). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `knope.toml` 08a547f0466a -> a0daf73a621a. Reason: publish-release.yml no longer runs knope release: its release job tags the verified commit and creates the GitHub Release through create-release.sh with a body capped under the API limit, and a workflow_dispatch recover job finishes a publish for an existing tag; knope.toml gained a comment only; knope-pin.test.sh no longer lists publish-release.yml as a knope user; checks.yml gained two additive test steps. What this record cites still holds, except that ADR-45 line 58 still says publish-release runs knope release, which is flagged for the maintainer to amend **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
