---
Stable-Id: 01M6SSWMQGGJMDPY49B3X47SVD
Status: Planned
Phase: '1'
Track: release-process
Depends-on: COND-5
Blocked-on: COND-5 -- the 0.4.0 release; its Signal says exactly what counts as met.
---
# 212 — Pin the gate and README to v0.4.0 and refresh known-refs after the release

## What

Bump the gate action's pinned release from `v0.3.0` to `v0.4.0`, bump the README pins to match (and fence the config example as `yaml`), and refresh `.urzua/known-refs.json`.

## Why

The gate action and the README pin `v0.3.0`, which cannot read the current configuration, and `known-refs.json` is a snapshot of 2026-11-04. The pins can only move to a release that exists, so this waits on the release (`COND-5`).

## Done means

- `.github/actions/urzua-gate/action.yml` and its self-test workflow default to `v0.4.0`, and the `gate (pinned release)` job passes against it.
- Every `v0.3.0` pin in `README.md` reads `v0.4.0`, and the config example is fenced `yaml`.
- `.urzua/known-refs.json` is regenerated and the record count is recorded in the revision row.
- `urzua check` and `check --base origin/main` report 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial record. **Why:** deferred during Phase 0 hardening and tracked by no record. | **structural** |
