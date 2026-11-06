---
Stable-Id: 01M6NX37Q0DCD64ASZAX6Y9DSA
Status: Done
Phase: '0'
Track: governance-process
Implements: ADR-89
Blocked-on: null
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/fix.rs, code:rust/crates/urzua-io/src/lib.rs, code:rust/crates/urzua-cli/src/commands/fix.rs, test:rust/crates/urzua-cli/tests/attestation_integration.rs
---
# 201 — Make the attestation identity source a configuration option and record it in the revision row

## What

Add the top-level `attestation` configuration (`identity: gh-verified | asserted`, `require_verified`) and make `urzua fix --apply` and `fix --rebaseline --apply` follow it, recording the identity source (`gh-verified`, `asserted` or `git-config`) and the actor kind (`agent`, `human` or `unspecified`, from a new `--actor` option) in the revision row. `ADR-89` is Accepted.

## Why

`resolve_identity` lets a verified `gh` login replace `--by`, naming the difference only in a `warning` field (`rust/crates/urzua-io/src/lib.rs:254-267`), and takes `--by` as given where `gh` is unavailable (`lib.rs:271-276`), so one command is verified in one environment and asserted in another and the revision row does not say which (`rust/crates/urzua-core/src/fix.rs:304`). Two re-pin reasons on 2026-11-05 did not describe the change they covered. A verified login names the account whose token ran the command, not whether a person or an agent held it, so under a shared token a `gh-verified` row names the maintainer as attester of an agent's work, the rubber stamp `RFC-2` and `BUG-16` describe; the actor kind makes that visible. `asserted` reverses the verified-beats-typed order of `RFC-2` and `MILE-66`, so it is opt-in and recorded as such (`MILE-18` is the longer-term identity binding). The maintainer decided on 2026-11-05 that the identity source is configuration, not a hardcoded rule.

## Done means

- `attestation.identity: gh-verified` (the default) keeps today's result byte for byte except that the report names the replaced `--by` as a field and the row carries `gh-verified`; with `require_verified: true` and no `gh` answer, `fix --apply` and `fix --rebaseline --apply` exit 2 with `not-run` and write nothing. Each is a test that fails before the change.
- `identity: asserted` records `--by` as given and the source `asserted`, with the verified login beside it when `gh` answers; `require_verified: true` with `asserted` is refused at load; an unknown `identity` value is a load error naming the allowed ones.
- `--actor agent|human` is accepted by both commands and the lifecycle commands pass `agent`; a run without it records `unspecified`; the label is documented as the caller's claim and not proof.
- The revision-row text records the source and the actor kind in every tool-authored row; existing rows are untouched, and a test pins both formats.
- This repository sets `attestation` in `.urzua/config.yaml`, a re-pin or a recompute in the PR that merges this milestone is written under it, and `urzua check` and `urzua check --base origin/main` report 0 unwaived errors or warnings.
- `SPEC-3` and `SPEC-8` document the option, its values, the row text and the load errors with revision rows; a minor changeset names the option.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
> | 2026-11-05 | Actor kind added to the attestation row and `RFC-2`, `BUG-16`, `MILE-18` and `MILE-66` cited, so an agent's re-pin under the maintainer's token is distinguishable from a person's; `asserted` is named as reversing the `RFC-2` and `MILE-66` order. **Why:** Independent review of pull request 355. Still `Planned`. | **substantive** |
> | 2026-11-06 | Built. `attestation` (`identity: gh-verified \| asserted`, `require_verified`) is read by `fix --apply` and `fix --rebaseline --apply`; every tool-authored row records the identity source and the actor kind from the new `--actor agent\|human` (`unspecified` when absent); the report gains `identity` naming a replaced `--by`; the invalid values are load errors; `SPEC-3` and `SPEC-8` document it and this repository declares `attestation`. `AGENTS.md` tells an agent to pass `--actor agent`; the lifecycle commands `/repin` and `/done` do not exist yet and are `MILE-199`'s, which is to pass it. `urzua new` still resolves its author under the default policy. **Why:** `ADR-89` section 5 (Accepted 2026-11-05); the PR delivers the `fix` side of every item of Done means. | **substantive** |
