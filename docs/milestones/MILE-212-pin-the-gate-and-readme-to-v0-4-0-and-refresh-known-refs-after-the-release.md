---
Stable-Id: 01M6SSWMQGGJMDPY49B3X47SVD
Status: Done
Phase: '1'
Track: release-process
Depends-on: COND-5
Blocked-on: null
---
# 212 — Pin the gate and README to v0.4.2 and refresh known-refs after the release

## What

Bump the gate action's pinned release from `v0.3.0` to `v0.4.2` (`v0.4.0` and `v0.4.1` were tagged without binaries, so `v0.4.2` is the first release that can be pinned), bump the README pins to match (and fence the config example as `yaml`), and refresh `.urzua/known-refs.json`.

## Why

The gate action and the README pin `v0.3.0`, which cannot read the current configuration, and `known-refs.json` is a snapshot of 2026-11-04. The pins can only move to a release that exists, so this waits on the release (`COND-5`).

## Done means

- `.github/actions/urzua-gate/action.yml` and its self-test workflow default to `v0.4.2`. — evidence: code:.github/actions/urzua-gate/action.yml@sha256:e08bf5875d469591c066388d89eea0b9078847e4f5a80d4765c5f4a034a88531, code:.github/workflows/ci.yml@sha256:b738f035984a2984511ce93cd77f71dbc2298d3ef1da3aadcb8d6750eea529a8
- The `gate (pinned release)` job passes against `v0.4.2`. — evidence: code:.github/workflows/ci.yml@sha256:b738f035984a2984511ce93cd77f71dbc2298d3ef1da3aadcb8d6750eea529a8
- Every `v0.3.0` pin in `README.md` reads `v0.4.2`. — evidence: code:README.md@sha256:3f99410b637433bafe383612dd5565edf7538e499e63d3797067428f271f90ed
- The `README.md` config example is fenced `yaml`. — evidence: code:README.md@sha256:3f99410b637433bafe383612dd5565edf7538e499e63d3797067428f271f90ed
- `.urzua/known-refs.json` is regenerated and the record count is recorded in the revision row. — evidence: code:.urzua/known-refs.json@sha256:b53b37b30931abf29b29cd97dee278167a3f7238b53a2582de67a75c539d0c96
- `urzua check` and `check --base origin/main` report 0 unwaived findings. — evidence: code:.github/workflows/ci.yml@sha256:b738f035984a2984511ce93cd77f71dbc2298d3ef1da3aadcb8d6750eea529a8

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial record. **Why:** deferred during Phase 0 hardening and tracked by no record. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-14 | Retargeted from `v0.4.0` to `v0.4.2`, and Done means now names `v0.4.2`. **Why:** `v0.4.0` and `v0.4.1` were tagged without binaries (the release notes were too long for GitHub, then the macOS runner image was retired), so only `v0.4.2` can be pinned. Pins moved in `README.md`, the gate action docs and `gate (pinned release)` (`ref`, `version`, `sha256`); `.urzua/known-refs.json` regenerated with 525 numbers (2 issues, 523 pulls). | **substantive** |
> | 2026-11-14 | `Blocked-on` cleared. **Why:** `COND-5` is `Met` by the `v0.4.2` release (page: https://github.com/urzuahq/urzua/releases/tag/v0.4.2, two attested archives); this record is the one that pins it, so `Met-by` on `COND-5` names it. | **structural** |
> | 2026-11-14 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (6 of 6 items resolved) computes to Implemented` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-15 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `README.md` af7e244845f6 -> 96aa9c8d86c6. Reason: README.md: the adopt block calls urzua from PATH and points at the install page instead of a clone and build; the pinned gate version line is unchanged, so the claim that the README pins the release still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `README.md` 96aa9c8d86c6 -> 3f99410b6374. Reason: README.md: the gate example's version line moved from v0.4.2 to v0.4.4, a later release; the README still pins an existing attested release rather than v0.3.0 and its config example is still fenced yaml, so the claim holds, with the v0.4.2 in this record's item superseded by MILE-286. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
