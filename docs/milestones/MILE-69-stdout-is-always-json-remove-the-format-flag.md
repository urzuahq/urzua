---
Status: Done
Stable-Id: 01M1YNNG9RQEZYDDHBQDN5AJDW
Phase: '0'
Track: governance-process
Implements: ADR-23
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:660eb55883131c1eab900d3a2a7ad5d93c5e0c6ace9c0f135f13ef83351f31fd, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5, test:rust/crates/urzua-cli/tests/check_integration.rs#a_bad_flag_emits_json_not_a_panic@sha256:eb8bfb4d74748038835831064ab072943e8c32648f99c1202940cb169993747a
---
# 69 — Stdout is always JSON; remove the --format flag

## What

Removed `--format`: `check`, `fix`, `explain`, `graph`, and `migrate schema --report` now emit exactly one JSON object on stdout, unconditionally, with no human-readable rendering anywhere.

## Why

Two output shapes mean two things that can silently drift apart, and an agent piping stdout, a script, and a person reading a terminal should all see exactly the same thing -- one shape is a stronger guarantee than a flag that happens to default correctly.

## Done means

- `check`, `fix`, `explain`, `graph` and `migrate schema --report` each emit exactly one JSON object on stdout, unconditionally. — evidence: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:660eb55883131c1eab900d3a2a7ad5d93c5e0c6ace9c0f135f13ef83351f31fd, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5
- There is no `--format` flag and no human-readable rendering. — evidence: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:660eb55883131c1eab900d3a2a7ad5d93c5e0c6ace9c0f135f13ef83351f31fd
- A test asserts the flag is rejected and stdout parses as one JSON value. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#a_bad_flag_emits_json_not_a_panic@sha256:eb8bfb4d74748038835831064ab072943e8c32648f99c1202940cb169993747a

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#emit` (lines 44-69) 29d071bbb823 -> 660eb5588313. Reason: Read the diff of rust/crates/urzua-cli/src/main.rs against main. emit now serializes through a private WithNextAction wrapper that appends next_action and next_action_reason after the report's own keys, and falls back to a could-not-run JSON with exit 2 when a report cannot serialize as an object; report_value (used by urzua mcp) is added; the panic hook JSON gains both keys as null (MILE-72). emit still writes one JSON document and otherwise returns the report's exit code, so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
