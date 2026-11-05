---
Stable-Id: 01M6NX37Q0DCD64ASZAX6Y9DSA
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-89
Blocked-on: null
---
# 201 — Make the attestation identity source a configuration option and record it in the revision row

## What

Add the top-level `attestation` configuration (`identity: gh-verified | asserted`, `require_verified`) and make `urzua fix --apply` and `fix --rebaseline --apply` follow it, recording the identity source (`gh-verified`, `asserted` or `git-config`) in the revision row. It is blocked until the maintainer accepts `ADR-89`.

## Why

`resolve_identity` lets a verified `gh` login replace `--by`, naming the difference only in a `warning` field (`rust/crates/urzua-io/src/lib.rs:254-267`), and takes `--by` as given where `gh` is unavailable (`lib.rs:271-276`), so one command is verified in one environment and asserted in another and the revision row does not say which (`rust/crates/urzua-core/src/fix.rs:304`). Two re-pin reasons on 2026-11-05 did not describe the change they covered. The maintainer decided on 2026-11-05 that the identity source is configuration, not a hardcoded rule.

## Done means

- `attestation.identity: gh-verified` (the default) keeps today's result byte for byte except that the report names the replaced `--by` as a field and the row carries `gh-verified`; with `require_verified: true` and no `gh` answer, `fix --apply` and `fix --rebaseline --apply` exit 2 with `not-run` and write nothing. Each is a test that fails before the change.
- `identity: asserted` records `--by` as given and the source `asserted`, with the verified login beside it when `gh` answers; `require_verified: true` with `asserted` is refused at load; an unknown `identity` value is a load error naming the allowed ones.
- The revision-row text records the source in every tool-authored row; existing rows are untouched, and a test pins both formats.
- This repository sets `attestation` in `.urzua/config.yaml`, a re-pin or a recompute in the PR that merges this milestone is written under it, and `urzua check` and `urzua check --base origin/main` report 0 unwaived errors or warnings.
- `SPEC-3` and `SPEC-8` document the option, its values, the row text and the load errors with revision rows; a minor changeset names the option.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
