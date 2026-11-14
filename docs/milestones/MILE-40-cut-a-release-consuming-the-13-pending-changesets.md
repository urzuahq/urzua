---
Status: Done
Stable-Id: 01M1YKX8WYPDEW5D4M1T1RHVB6
Phase: '1'
Track: release-process
Implements: ADR-45
Blocked-on: —
Realized-by: code:.github/workflows/prepare-release.yml@sha256:0e4d28280d1a77819841035b7ed7da5ee570cf96e80875d2f1e8cc83d7596b8d, code:.github/workflows/publish-release.yml@sha256:5aaa0797cb94173cbbb2de30c7d2a3c3160476c9a54b2763fae8b76ed1c5fe0e, code:knope.toml@sha256:a0daf73a621a5ddd0e1e9735e6a324179ec9754573ebc4146e7631f25b1d9bc4
---
# 40 — Cut a release consuming the pending changesets

## What

_Original plan, written while `ADR-29`'s manual `workflow_dispatch` release was in force. `ADR-45` has since replaced it with a reviewable release PR; this milestone now records that as its `Implements`._

Trigger the existing `prepare-release.yml` `workflow_dispatch` to consume all pending
`.changeset/*.md` fragments (14 as of 2026-09-07, up from 13 when this milestone was first written
-- the exact count drifts as work ships, and isn't restated as a fixed number in the title to avoid
that drift making the title itself stale), bump `rust/Cargo.toml`'s version, and compile the real
`CHANGELOG.md` entries for the work shipped since `v0.1.0` (2026-09-05): `fix`, `audit`, `migrate
ids`, `explain`/`graph`, drift detection, header-shape rules, identity verification, the filename
type-prefix + no-padding rename, and the `mile` prefix change.

## Why

`CHANGELOG.md`'s existing `0.1.0` entry says `new`, `audit`, `migrate`, `export`, and `import` all
"exit 2 with not implemented yet" -- false today, since README's own Status table lists all but
`export`/`import` as real. Anyone reading `CHANGELOG.md` right now gets a wrong picture of what the
tool currently does. This is a maintainer-triggered act (ADR-29) -- tracked here rather than run
unilaterally.

## Done means

- A release past `v0.1.0` was cut that consumed the pending `.changeset/*.md` fragments, bumped `rust/Cargo.toml`'s version and compiled `CHANGELOG.md` entries for the work shipped since (`CHANGELOG.md` carries `0.2.0`, `0.2.1` and `0.3.0`). — waived: process-outcome -- the release is a historical event recorded by the tags `v0.2.0`, `v0.2.1` and `v0.3.0`, not by a file: `CHANGELOG.md` and `rust/Cargo.toml` are rewritten by every later release, so a pin on either goes stale at each one
- The release went through the release mechanism `ADR-45` decided (`prepare-release.yml` then `publish-release.yml`), not the `workflow_dispatch` path first planned. — evidence: code:.github/workflows/prepare-release.yml@sha256:0e4d28280d1a77819841035b7ed7da5ee570cf96e80875d2f1e8cc83d7596b8d, code:.github/workflows/publish-release.yml@sha256:5aaa0797cb94173cbbb2de30c7d2a3c3160476c9a54b2763fae8b76ed1c5fe0e, code:knope.toml@sha256:a0daf73a621a5ddd0e1e9735e6a324179ec9754573ebc4146e7631f25b1d9bc4

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | `Implements` repointed from `ADR-29` to `ADR-45`. **Why:** `ADR-29` is Superseded and `ADR-45` is the decision now in force for this work; `pointer.target-status` flagged the stale target. | **structural** |
> | 2026-09-16 | `Planned` -> `Done`. **Why:** done twice -- `v0.2.0` consumed twenty accumulated fragments into a 14,380-character changelog, and `v0.2.1` consumed one. The stale `0.1.0` changelog entry this milestone was filed against is no longer the top of the file. Cut through `ADR-45`'s reviewable release PR rather than `ADR-29`'s `workflow_dispatch`, which no longer exists. | **substantive** |
> | 2026-09-07 | Dropped the hardcoded "13" from the title (now "the pending changesets"); noted the count is 14 as of this date in the body instead. **Why:** found live during backlog triage -- the count had already drifted from 13 to 14 with nothing catching it, and hardcoding a transient number into a permanent title guarantees it drifts again. | **structural** |
> | 2026-11-02 | Labelled the body as the original `ADR-29`-era plan. **Why:** review found the body still described a release flow `ADR-45` replaced. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 5 locators pinned, 5 file-level, 0 scoped. Evidence: CHANGELOG.md@sha256:ba2c27b0cf5b, rust/Cargo.toml@sha256:e3fe29ad3d95, .github/workflows/prepare-release.yml@sha256:0e4d28280d1a, .github/workflows/publish-release.yml@sha256:530bdeeececf, knope.toml@sha256:08a547f0466a Reason: Pinned the files as they are now after checking each against the record it evidences: schema_version.rs, commands/migrate.rs, table.rs and config.yaml still carry the schema version window and the cross-record rules (MILE-34), AGENTS.md still instructs agents to track work in the record types, verify against real runs, never silently rewrite an Accepted decision and not build speculative capability (MILE-36), prepare-release.yml still triggers only on push to main and opens a release pull request with knope.toml and release-guard.sh (MILE-39), section.rs still implements required sections and the y-statement shape (MILE-4, MILE-5), CHANGELOG.md, rust/Cargo.toml and the release workflows still show releases past v0.1.0 consuming changesets (MILE-40), report.rs, action.rs, table.rs and generate-rule-table.py still give every finding a suggested action (MILE-44), report.rs, config.rs and runner.rs still carry the info level (MILE-46), document_model.rs still reads a second corpus's declared model (MILE-51), and the Phase A rule modules, config.yaml and checks.yml still hold the header, pointer, field, filename and supersession rules (MILE-56). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Done means item 1 moved from evidence on `CHANGELOG.md` and `rust/Cargo.toml` to a `process-outcome` waiver naming the tags `v0.2.0`, `v0.2.1` and `v0.3.0`, and both locators removed from `Realized-by`. **Why:** the two files are rewritten by every release, so each release left this milestone's pins stale and the first claim expiry after it needed a manual re-pin, for a fact (a release past `v0.1.0` was cut) that no later release can change. The mechanism evidence of item 2 (`prepare-release.yml`, `publish-release.yml`, `knope.toml`) is unchanged. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/publish-release.yml` 530bdeeececf -> 5aaa0797cb94, `knope.toml` 08a547f0466a -> a0daf73a621a. Reason: publish-release.yml no longer runs knope release: its release job tags the verified commit and creates the GitHub Release through create-release.sh with a body capped under the API limit, and a workflow_dispatch recover job finishes a publish for an existing tag; knope.toml gained a comment only; knope-pin.test.sh no longer lists publish-release.yml as a knope user; checks.yml gained two additive test steps. What this record cites still holds, except that ADR-45 line 58 still says publish-release runs knope release, which is flagged for the maintainer to amend **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
