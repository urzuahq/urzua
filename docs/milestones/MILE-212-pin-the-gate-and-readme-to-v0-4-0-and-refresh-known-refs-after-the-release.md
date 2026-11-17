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

- `.github/actions/urzua-gate/action.yml` and its self-test workflow default to `v0.4.2`. — evidence: code:.github/actions/urzua-gate/action.yml@sha256:efb407fc8dfe418e800af79badcb3d0d9db8464c7ecc42e6016eff4772ae1f25, code:.github/workflows/ci.yml@sha256:d967686a97628663a0fcd754ce5da6e83f38a374d9f784fa7f06d9af26f67563
- The `gate (pinned release)` job passes against `v0.4.2`. — evidence: code:.github/workflows/ci.yml@sha256:d967686a97628663a0fcd754ce5da6e83f38a374d9f784fa7f06d9af26f67563
- Every `v0.3.0` pin in `README.md` reads `v0.4.2`. — evidence: code:README.md@sha256:a5d3b0024473c37e40332e124369ad3fc652d1356cfe65bf535b4da008499f8c
- The `README.md` config example is fenced `yaml`. — evidence: code:README.md@sha256:a5d3b0024473c37e40332e124369ad3fc652d1356cfe65bf535b4da008499f8c
- `.urzua/known-refs.json` is regenerated and the record count is recorded in the revision row. — evidence: code:.urzua/known-refs.json@sha256:b53b37b30931abf29b29cd97dee278167a3f7238b53a2582de67a75c539d0c96
- `urzua check` and `check --base origin/main` report 0 unwaived findings. — evidence: code:.github/workflows/ci.yml@sha256:d967686a97628663a0fcd754ce5da6e83f38a374d9f784fa7f06d9af26f67563

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
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/actions/urzua-gate/action.yml` e08bf5875d46 -> c561bb1562ab, `.github/workflows/ci.yml` b738f035984a -> d6fdad2c5ec6. Reason: the pinned release moves from v0.4.2 to v0.4.4 in this file (version, ref and the linux archive sha256 in the pinned-release gate job, and the version named in the action's examples); nothing else in the file changed and the claim still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `README.md` 3f99410b6374 -> 241f29aa390f. Reason: Read the diff of README.md: the only change is the Adopt without a rewrite sentence, which now says docs/reference/install.md has a one-line install script and the manual steps; the gate and release pins and the yaml fence this record cites are untouched, so the claim still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/ci.yml` d6fdad2c5ec6 -> fe22a1814fb9. Reason: ci.yml gains the install-script job, which runs scripts/install-test.sh, and the ci job now requires it; the jobs this record relies on are unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `README.md` 241f29aa390f -> a64c9dc99705. Reason: README's fix, re-pin and renew commands now carry --actor agent and the placeholder convention; the record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/actions/urzua-gate/action.yml` c561bb1562ab -> 0efdd1aedd9e, `.github/workflows/ci.yml` fe22a1814fb9 -> 4ea9e5c7cf7d, `README.md` a64c9dc99705 -> 2b0c71caaac8. Reason: the pinned release moves from v0.4.4 to v0.4.5 in this file (version, ref and the linux archive sha256 in the pinned-release gate job, the version named in the action's examples, the README's gate example); nothing else in the file changed and the claim still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/actions/urzua-gate/action.yml` 0efdd1aedd9e -> 9cd5f08b31be, `.github/workflows/ci.yml` 4ea9e5c7cf7d -> 0912db414050, `README.md` 2b0c71caaac8 -> 40f0555437dd. Reason: the pinned release moves from v0.4.5 to v0.4.6 in this file (version, ref and the linux archive sha256 in the pinned-release gate job, the version named in the action's examples, the README's gate example); nothing else in the file changed and the claim still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/actions/urzua-gate/action.yml` 9cd5f08b31be -> efb407fc8dfe, `.github/workflows/ci.yml` 0912db414050 -> d967686a9762, `README.md` 40f0555437dd -> 7b481e5296e4. Reason: ci.yml: the pinned-release gate job's ref, version and linux archive sha256 move from v0.4.6 to v0.4.7; nothing else changed. action.yml: the version named in the usage example and the input description moves to v0.4.7. README.md: the gate example's version moves to v0.4.7. The claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `README.md` 7b481e5296e4 -> a5d3b0024473. Reason: README.md: gains a 'Who uses urzua' section naming three repositories and showing one adopter's board; the gate example and its pinned version are unchanged. The claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
