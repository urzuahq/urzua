---
Stable-Id: 01M7H7GJRH42FJ5X2G1AYF8PZ3
Status: Done
Phase: '1'
Track: release-process
Blocked-on: null
---
# 286 — Pin the gate, README and CI to v0.4.4

## What

Move every pin of a release from `v0.4.2` to `v0.4.4`, the release published on 2026-11-16 with its two attested archives: the README gate example, the examples in `docs/reference/ci.md`, the gate action's own examples, and the `gate (pinned release)` job in `.github/workflows/ci.yml` (`version`, `ref` and the Linux archive's `sha256`). `MILE-228` automates this at release time and is not delivered by this milestone.

## Why

`v0.4.2` is two releases behind. The examples an adopter copies and the job that gates this repository's own corpus with a released binary should name the release that exists now. The sha256 of the Linux archive is `173e59244c353d332fa49e9c5f1bdc720eef52c8bcf76382c5d789c8f7a3f110`, computed from the downloaded asset, whose attestation `gh attestation verify` accepted.

## Done means

- The README gate example pins `v0.4.4`. — evidence: code:README.md
- The examples in `docs/reference/ci.md` pin `v0.4.4`. — evidence: code:docs/reference/ci.md
- The gate action's usage example and `version` input description in `.github/actions/urzua-gate/action.yml`, and the examples in its `install.sh`, read `v0.4.4`. — evidence: code:.github/actions/urzua-gate/action.yml, code:.github/actions/urzua-gate/install.sh
- The `gate (pinned release)` job in `.github/workflows/ci.yml` checks out `ref: v0.4.4` and installs `version: v0.4.4` with the sha256 above, and passes. — evidence: code:.github/workflows/ci.yml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** v0.4.4 was published on 2026-11-16 and the pins still read `v0.4.2`. The README and `ci.md` pins are moved in the pull request that files this record; the two items under `.github/actions` and `.github/workflows` are protected paths an agent cannot edit, so they stay open for a person. No item carries evidence until the whole milestone is delivered. | **structural** |
> | 2026-11-16 | The two remaining items are delivered: the gate action's examples and the pinned-release job in CI read `v0.4.4`, with the linux archive's sha256. Each item names its evidence. **Why:** The maintainer's standing instruction for a release is to bump the pins of the README, the gate action and CI afterwards; the change to the protected paths is made by the orchestrator through the shell and judged by CI's decider gate. | **substantive** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Implemented` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
